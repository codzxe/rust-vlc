//! The minimal in-tree MP4 muxer (card 7.E2, issue #44) — an
//! ISO-BMFF **container writer** for stream copy, never a transcode:
//! H.264 and AAC sample payloads flow through byte-verbatim, the
//! sample tables describe them, and every other codec is a typed
//! refusal. No dependency, no unsafe (C5); the box grammar is the
//! public ISO/IEC 14496-12 structure, clean-room from the demux
//! side's own reader (`demux::mp4`) and the committed corpus's
//! observed shapes.
//!
//! # The file written (progressive, non-fragmented)
//!
//! ```text
//! ftyp (isom / isom, iso2, avc1, mp41)
//! mdat (size patched at finalize; sample bytes streamed verbatim)
//! moov
//!   mvhd (movie timescale 1000)
//!   trak ×N
//!     tkhd (flags 0x7, movie-timescale duration)
//!     mdia
//!       mdhd (the track's own timescale, v1 when duration > u32)
//!       hdlr (vide / soun)
//!       minf
//!         vmhd | smhd
//!         dinf > dref > url  (self-contained)
//!         stbl
//!           stsd (ONE entry: the source's sample-entry payload, verbatim)
//!           stts (run-length sample durations)
//!           ctts (only when some composition offset is nonzero;
//!                 version 1 when any offset is negative)
//!           stss (video only, when not every sample is sync;
//!                 audio omits it — absence means all-sync)
//!           stsc (one run: every chunk carries one sample)
//!           stsz (per-sample sizes)
//!           stco (per-sample chunk offsets)
//! ```
//!
//! One sample per chunk keeps the interleave free: samples may
//! arrive in any cross-track order (the demuxer's file order is the
//! natural one) and the per-sample `stco` table points at wherever
//! each landed in `mdat`.
//!
//! # What is copied verbatim, and what is rebuilt (honesty table)
//!
//! * **Sample payload bytes** — streamed to `mdat` untouched.
//! * **The `stsd` entry payload** — the host hands the source
//!   demuxer's verbatim sample-entry bytes (e.g.
//!   [`Mp4Demuxer::sample_entry_bytes`](demux::mp4::Mp4Demuxer::sample_entry_bytes)):
//!   the `avcC`/`esds` config rides along byte-identically. The
//!   entry's fourcc is re-derived from the track's codec descriptor
//!   (`h264` → `avc1`, `aac` → `mp4a`) — the same mapping the demux
//!   side applied in reverse.
//! * **The timing tables are rebuilt from the packets** — per-sample
//!   DTS deltas, composition offsets (`pts − dts`), sync flags and
//!   sizes are the source's own values re-serialized, not its table
//!   bytes (a fresh `moov` must describe a fresh `mdat` layout).
//! * **The movie timeline is re-anchored at each track's first
//!   sample DTS**: a fresh movie header carries no edit lists, so
//!   the output's media timeline starts at the first sample. The
//!   source's surfaced DTS can be nonzero or negative (the demux
//!   side applies `elst` edits — an encoder-delay trim surfaces as a
//!   negative first DTS); the copy anchors that sample at tick 0 and
//!   every later stamp shifts with it (composition offsets `pts−dts`
//!   ride unchanged), which is what makes the round-trip suites'
//!   re-anchored stamp comparisons exact.
//!
//! # Typed ceilings (MP §12 — bounded, never wrapped)
//!
//! * per-track sample count ≤ [`MAX_REMUX_SAMPLES`] (the demux
//!   layer's own sample ceiling, mirrored — a 1 Mi-sample table stays
//!   under the demuxer's 16 MiB `moov` parse ceiling at this muxer's
//!   ~10 bytes per sample);
//! * sample-entry bytes ≤ [`MAX_SAMPLE_ENTRY_BYTES`];
//! * total `mdat` payload and every `stco` offset must stay inside
//!   the classic 32-bit fields — [`mp4_size_overflow`] past the
//!   ceiling (the WAV RIFF-ceiling precedent; `co64`/`largesize`
//!   belong to a later card);
//! * the `moov` is built in a `Vec<u8>` sized by those same bounded
//!   tables, then written once — no second pass over the sink.
//!
//! # Seekability contract
//!
//! `mdat`'s size field precedes its payload, so the writer patches
//! it at [`finalize`](Mp4Remuxer::finalize) — the sink must be
//! `Write + Seek` ([`WriteSeek`]). The 7.E3 raw-ES pipe sink is the
//! write-only surface; MP4 output to a pipe is honestly NOT offered
//! (a seek-less MP4 would need `mdat` size 0 = to-EOF, which swallows
//! the trailing `moov` — a file our own demuxer refuses).

use std::io::{Seek, SeekFrom, Write};

use media_types::EngineError;
use media_types::ids::MediaKind;
use media_types::ids::TrackId;
use media_types::packet::SharedBytes;
use media_types::time::Stamp;

use crate::error::{
    mp4_dts_non_monotonic, mp4_samples_over_budget, mp4_size_overflow, mp4_stamp_missing,
    mp4_track_index, sink_seek_io, sink_write_io,
};
use crate::graph::WriteSeek;

/// Maximum samples per remux track `[PROPOSAL: 1 Mi — the demux
/// layer's own MAX_SAMPLES, mirrored; the moov built from a
/// maximal table stays under its 16 MiB parse ceiling]`.
pub const MAX_REMUX_SAMPLES: usize = 1024 * 1024;

/// Maximum sample-entry bytes per remux track config `[PROPOSAL:
/// 64 KiB — real avcC/esds entries are tens-to-hundreds of bytes;
/// the ceiling bounds the config, not the media]`.
pub const MAX_SAMPLE_ENTRY_BYTES: usize = 64 * 1024;

/// The movie timescale every `mvhd`/`tkhd` duration is expressed in
/// (milliseconds — the ffmpeg convention).
const MOVIE_TIMESCALE: u32 = 1000;

/// `tkhd` flags: track_enabled | track_in_movie.
const TKHD_FLAGS_ENABLED: u32 = 0x7;

/// One track's host-harvested config: which source track, and its
/// verbatim sample-entry payload (the `stsd` entry body after the
/// 8-byte size+fourcc header — exactly what
/// [`Mp4Demuxer::sample_entry_bytes`](demux::mp4::Mp4Demuxer::sample_entry_bytes)
/// returns).
#[derive(Debug, Clone)]
pub struct Mp4TrackConfig {
    /// The source demuxer's track handle this config serves.
    pub track: TrackId,
    /// The verbatim sample-entry payload (codec config rides inside).
    pub sample_entry: SharedBytes,
}

/// One track's complete setup for [`Mp4Remuxer::new`]: the
/// host-harvested config plus the track-table facts the runner
/// cross-checks (codec descriptor, kind, media timescale). Hosts
/// driving the muxer directly supply the same five values.
#[derive(Debug, Clone)]
pub struct Mp4TrackSetup {
    /// The host-harvested config (track handle + sample entry).
    pub config: Mp4TrackConfig,
    /// The source track's codec descriptor (`h264` / `aac`).
    pub codec: String,
    /// What the track carries (drives `hdlr`/`vmhd`/`smhd`).
    pub kind: MediaKind,
    /// The track's media timescale (its `TrackTiming` base, which
    /// must be the native `1/timescale` form — the runner refuses
    /// anything else typed before the muxer is built).
    pub timescale: u32,
}

/// The fourcc this muxer writes for a codec descriptor (the demux
/// side's mapping, inverted). `None` = outside the stream-copy set.
#[must_use]
pub fn sample_entry_fourcc(codec: &str, kind: MediaKind) -> Option<&'static [u8; 4]> {
    match (kind, codec) {
        (MediaKind::Video, "h264") => Some(b"avc1"),
        (MediaKind::Audio, "aac") => Some(b"mp4a"),
        _ => None,
    }
}

/// One recorded sample (the bounded per-track table row).
struct SampleRow {
    /// Absolute file offset of the sample's bytes in `mdat`.
    offset: u32,
    /// Payload size.
    size: u32,
    /// DTS on the re-anchored media timeline (ticks).
    dts: u64,
    /// Composition offset `pts − dts` (ticks; may be negative).
    cts: i64,
    /// Whether the sample starts a keyframe.
    sync: bool,
}

/// One track's accumulating state.
struct TrackState {
    setup: Mp4TrackSetup,
    rows: Vec<SampleRow>,
    /// The re-anchor origin (the first sample's raw DTS).
    origin: Option<i64>,
    /// The most recent packet-declared duration (ticks) — the final
    /// sample's `stts` delta, falling back to the previous delta
    /// when the container declared none.
    last_duration: u64,
}

/// The streaming MP4 remux writer (module docs). Build with
/// [`Mp4Remuxer::new`], feed every selected track's packets through
/// [`add_packet`](Mp4Remuxer::add_packet) in any cross-track order,
/// then [`finalize`](Mp4Remuxer::finalize).
///
/// Not `Clone`: one writer owns one output.
pub struct Mp4Remuxer {
    out: Box<dyn WriteSeek>,
    tracks: Vec<TrackState>,
    /// Absolute file offset where `mdat`'s size field lives.
    mdat_size_at: u64,
    /// Absolute file offset of the first `mdat` payload byte.
    mdat_payload_at: u64,
    /// `mdat` payload bytes streamed so far (the future size − 8).
    mdat_bytes: u64,
}

impl Mp4Remuxer {
    /// Assemble the writer and emit `ftyp` + the `mdat` header.
    ///
    /// # Errors
    /// [`sink_write_io`]/[`sink_seek_io`] when the sink fails.
    pub fn new(
        out: Box<dyn WriteSeek>,
        setups: Vec<Mp4TrackSetup>,
    ) -> Result<Mp4Remuxer, EngineError> {
        let tracks = setups
            .into_iter()
            .map(|setup| TrackState {
                setup,
                rows: Vec::new(),
                origin: None,
                last_duration: 0,
            })
            .collect();
        let mut muxer = Mp4Remuxer {
            out,
            tracks,
            mdat_size_at: 0,
            mdat_payload_at: 0,
            mdat_bytes: 0,
        };
        muxer.write_ftyp()?;
        muxer.write_mdat_header()?;
        Ok(muxer)
    }

    /// Stream one packet of track `index` (the setup ordinal) into
    /// `mdat` and record its table row. Payload bytes are written
    /// verbatim; nothing about the sample is transformed.
    ///
    /// # Errors
    /// Typed at stage `Transcode`: [`mp4_track_index`](crate::error::mp4_track_index)
    /// for an unknown setup ordinal, [`mp4_stamp_missing`](crate::error::mp4_stamp_missing)
    /// when the packet carries no DTS, [`mp4_dts_non_monotonic`](crate::error::mp4_dts_non_monotonic)
    /// when a track's DTS goes backwards, [`mp4_samples_over_budget`](crate::error::mp4_samples_over_budget)
    /// past the table ceiling, [`mp4_size_overflow`](crate::error::mp4_size_overflow)
    /// past the 32-bit `mdat`/`stco` reach, [`sink_write_io`] when
    /// the sink fails.
    pub fn add_packet(
        &mut self,
        index: usize,
        packet: &media_types::Packet,
    ) -> Result<u64, EngineError> {
        let state = self.tracks.get_mut(index).ok_or_else(mp4_track_index)?;
        let raw_dts = match packet.dts {
            Stamp::Present(stamp) => stamp.ticks,
            Stamp::Missing => return Err(mp4_stamp_missing()),
        };
        // A missing PTS is data: no composition offset is claimed.
        let cts = match packet.pts {
            Stamp::Present(pts) => pts.ticks.saturating_sub(raw_dts),
            Stamp::Missing => 0,
        };
        // Re-anchor: the first sample defines the media timeline 0.
        let origin = *state.origin.get_or_insert(raw_dts);
        let dts_signed = raw_dts.checked_sub(origin).ok_or(mp4_dts_non_monotonic())?;
        if let Some(last) = state.rows.last() {
            let prev = i64::try_from(last.dts).unwrap_or(i64::MAX);
            if dts_signed < prev {
                return Err(mp4_dts_non_monotonic());
            }
        } else if dts_signed != 0 {
            // Unreachable by construction (origin == raw_dts here);
            // the typed answer, never a wrapped table.
            return Err(mp4_dts_non_monotonic());
        }
        if state.rows.len() >= MAX_REMUX_SAMPLES {
            return Err(mp4_samples_over_budget(
                state.rows.len() + 1,
                MAX_REMUX_SAMPLES,
            ));
        }
        let size = u32::try_from(packet.data.len()).map_err(|_| mp4_size_overflow(u64::MAX))?;
        let after = self.mdat_bytes + u64::from(size);
        // The classic 32-bit ceilings: the mdat extent and every
        // stco offset (the payload's base is a file position too).
        if after + 8 > u64::from(u32::MAX) || self.mdat_payload_at + after > u64::from(u32::MAX) {
            return Err(mp4_size_overflow(self.mdat_payload_at + after));
        }
        // Track the declared duration (nanos → ticks, nearest).
        if let Some(media_types::time::Duration::Known(nanos)) = packet.duration {
            state.last_duration = nanos_to_ticks(nanos.0, state.setup.timescale);
        }
        let offset = self.mdat_payload_at + self.mdat_bytes;
        let offset = u32::try_from(offset).map_err(|_| mp4_size_overflow(offset))?;
        state.rows.push(SampleRow {
            offset,
            size,
            dts: u64::try_from(dts_signed).map_err(|_| mp4_dts_non_monotonic())?,
            cts,
            sync: packet.keyframe,
        });
        write_all(&mut self.out, packet.data.as_slice())?;
        self.mdat_bytes = after;
        Ok(u64::from(size))
    }

    /// Patch `mdat`'s size and write `moov`. Consumes the writer and
    /// flushes the sink.
    ///
    /// # Errors
    /// [`sink_seek_io`]/[`sink_write_io`] on sink failure;
    /// [`mp4_size_overflow`](crate::error::mp4_size_overflow) if the
    /// `mdat` extent cannot fit the 32-bit size field (the per-add
    /// checks make this a defensive impossibility, kept typed).
    pub fn finalize(mut self) -> Result<(), EngineError> {
        // Patch the mdat size (header + payload).
        seek_to(&mut self.out, self.mdat_size_at)?;
        let mdat_size = self
            .mdat_bytes
            .checked_add(8)
            .filter(|s| *s <= u64::from(u32::MAX))
            .ok_or_else(|| mp4_size_overflow(self.mdat_bytes + 8))?;
        write_all(&mut self.out, &(mdat_size as u32).to_be_bytes())?;
        // moov goes after the payload.
        seek_to(&mut self.out, self.mdat_payload_at + self.mdat_bytes)?;
        let moov = self.build_moov();
        write_all(&mut self.out, &moov)?;
        self.out.flush().map_err(sink_write_io)?;
        Ok(())
    }

    /// `ftyp`: isom major, the compat set our own demuxer claims.
    fn write_ftyp(&mut self) -> Result<(), EngineError> {
        let mut buf = Vec::with_capacity(32);
        put_box(&mut buf, b"ftyp", &mut |body| {
            body.extend_from_slice(b"isom");
            body.extend_from_slice(&0u32.to_be_bytes());
            body.extend_from_slice(b"isom");
            body.extend_from_slice(b"iso2");
            body.extend_from_slice(b"avc1");
            body.extend_from_slice(b"mp41");
        });
        write_all(&mut self.out, &buf)
    }

    /// The `mdat` header with a placeholder size (patched at
    /// finalize); records the absolute payload offset.
    fn write_mdat_header(&mut self) -> Result<(), EngineError> {
        let here = self.out.stream_position().map_err(sink_seek_io)?;
        self.mdat_size_at = here;
        write_all(&mut self.out, &0u32.to_be_bytes())?;
        write_all(&mut self.out, b"mdat")?;
        let payload_at = self.out.stream_position().map_err(sink_seek_io)?;
        self.mdat_payload_at = payload_at;
        Ok(())
    }

    /// Build the whole `moov` in memory (bounded by the table
    /// ceilings) and return its bytes — one container box around the
    /// `mvhd` and every `trak`.
    fn build_moov(&self) -> Vec<u8> {
        let mut children = Vec::new();
        let movie_durs: Vec<u64> = self.tracks.iter().map(track_movie_duration).collect();
        let max_movie_dur = movie_durs.iter().copied().max().unwrap_or(0);
        put_box(&mut children, b"mvhd", &mut |body| {
            put_mvhd(body, max_movie_dur, self.tracks.len() + 1);
        });
        for (ti, state) in self.tracks.iter().enumerate() {
            put_box(&mut children, b"trak", &mut |trak| {
                put_box(trak, b"tkhd", &mut |body| {
                    put_tkhd(body, ti as u64 + 1, movie_durs[ti], state);
                });
                put_box(trak, b"mdia", &mut |mdia| {
                    put_box(mdia, b"mdhd", &mut |body| {
                        put_mdhd(body, state);
                    });
                    put_box(mdia, b"hdlr", &mut |body| {
                        put_hdlr(body, state.setup.kind);
                    });
                    put_box(mdia, b"minf", &mut |minf| {
                        if state.setup.kind == MediaKind::Video {
                            put_box(minf, b"vmhd", &mut |body| {
                                body.extend_from_slice(&0u32.to_be_bytes());
                                body.extend_from_slice(&0u16.to_be_bytes());
                                body.extend_from_slice(&[0, 0, 0]);
                            });
                        } else {
                            put_box(minf, b"smhd", &mut |body| {
                                body.extend_from_slice(&0u32.to_be_bytes());
                                body.extend_from_slice(&0u16.to_be_bytes());
                                body.extend_from_slice(&0u16.to_be_bytes());
                            });
                        }
                        put_box(minf, b"dinf", &mut |dinf| {
                            put_box(dinf, b"dref", &mut |dref| {
                                dref.extend_from_slice(&0u32.to_be_bytes());
                                dref.extend_from_slice(&1u32.to_be_bytes());
                                put_box(dref, b"url ", &mut |url| {
                                    // flags=1: the data is in this file.
                                    url.extend_from_slice(&1u32.to_be_bytes());
                                });
                            });
                        });
                        put_box(minf, b"stbl", &mut |stbl| {
                            put_stbl(stbl, state);
                        });
                    });
                });
            });
        }
        // The one wrapper: `moov` around its `mvhd` + `trak` children
        // (a bare flat child sequence is not a movie — the demuxer's
        // top-level scan looks for the container, and the round-trip
        // suite pins it).
        let mut moov = Vec::new();
        put_box(&mut moov, b"moov", &mut |body| {
            body.extend_from_slice(&children);
        });
        moov
    }
}

/// `mvhd` v0 (movie timescale 1000; durations in movie units).
fn put_mvhd(body: &mut Vec<u8>, movie_dur: u64, next_track: usize) {
    body.extend_from_slice(&0u32.to_be_bytes()); // version/flags
    body.extend_from_slice(&0u32.to_be_bytes()); // creation
    body.extend_from_slice(&0u32.to_be_bytes()); // modification
    body.extend_from_slice(&MOVIE_TIMESCALE.to_be_bytes());
    put_u32_saturating(body, movie_dur);
    body.extend_from_slice(&0x0001_0000u32.to_be_bytes()); // rate 1.0
    body.extend_from_slice(&0x0100u16.to_be_bytes()); // volume 1.0
    body.extend_from_slice(&[0u8; 10]); // reserved
    put_identity_matrix(body);
    body.extend_from_slice(&[0u8; 24]); // pre_defined
    let next = u32::try_from(next_track).unwrap_or(u32::MAX);
    body.extend_from_slice(&next.to_be_bytes());
}

/// `tkhd` v0 (flags 0x7; width/height parsed from a video entry).
fn put_tkhd(body: &mut Vec<u8>, track_id: u64, movie_dur: u64, state: &TrackState) {
    body.extend_from_slice(&TKHD_FLAGS_ENABLED.to_be_bytes());
    body.extend_from_slice(&0u32.to_be_bytes()); // creation
    body.extend_from_slice(&0u32.to_be_bytes()); // modification
    put_u32_saturating(body, track_id);
    body.extend_from_slice(&0u32.to_be_bytes()); // reserved
    put_u32_saturating(body, movie_dur);
    body.extend_from_slice(&[0u8; 8]); // reserved
    body.extend_from_slice(&0u16.to_be_bytes()); // layer
    body.extend_from_slice(&0u16.to_be_bytes()); // alternate_group
    let volume: u16 = if state.setup.kind == MediaKind::Video {
        0
    } else {
        0x0100
    };
    body.extend_from_slice(&volume.to_be_bytes());
    body.extend_from_slice(&0u16.to_be_bytes()); // reserved
    put_identity_matrix(body);
    // 16.16 width/height: video from the sample entry's own u16
    // fields (payload offsets 24/26); everything else zero.
    let (w, h) = video_dimensions(state.setup.config.sample_entry.as_slice());
    body.extend_from_slice(&(u32::from(w) << 16).to_be_bytes());
    body.extend_from_slice(&(u32::from(h) << 16).to_be_bytes());
}

/// `mdhd` (v0 while the duration fits u32, else v1 with 64-bit
/// fields — the demux side reads both).
fn put_mdhd(body: &mut Vec<u8>, state: &TrackState) {
    let dur = track_media_duration(state);
    if let Ok(dur32) = u32::try_from(dur) {
        body.extend_from_slice(&0u32.to_be_bytes()); // v0
        body.extend_from_slice(&0u32.to_be_bytes());
        body.extend_from_slice(&0u32.to_be_bytes());
        body.extend_from_slice(&state.setup.timescale.to_be_bytes());
        body.extend_from_slice(&dur32.to_be_bytes());
    } else {
        body.extend_from_slice(&0x0100_0000u32.to_be_bytes()); // v1
        body.extend_from_slice(&0u64.to_be_bytes());
        body.extend_from_slice(&0u64.to_be_bytes());
        body.extend_from_slice(&state.setup.timescale.to_be_bytes());
        body.extend_from_slice(&dur.to_be_bytes());
    }
    body.extend_from_slice(&0x55C4u16.to_be_bytes()); // language 'und'
    body.extend_from_slice(&0u16.to_be_bytes()); // pre_defined
}

/// `hdlr` with the kind's handler fourcc and an empty name.
fn put_hdlr(body: &mut Vec<u8>, kind: MediaKind) {
    body.extend_from_slice(&0u32.to_be_bytes()); // version/flags
    body.extend_from_slice(&0u32.to_be_bytes()); // pre_defined
    body.extend_from_slice(if kind == MediaKind::Video {
        b"vide"
    } else {
        b"soun"
    });
    body.extend_from_slice(&[0u8; 12]); // reserved
    body.push(0); // name: empty, null-terminated
}

/// The `stbl` family (module docs' honesty table).
fn put_stbl(stbl: &mut Vec<u8>, state: &TrackState) {
    let rows = &state.rows;
    // stsd: one entry — the verbatim payload under the derived fourcc.
    put_box(stbl, b"stsd", &mut |body| {
        body.extend_from_slice(&0u32.to_be_bytes()); // version/flags
        body.extend_from_slice(&1u32.to_be_bytes()); // entry count
        // The runner refused non-h264/aac before the muxer was
        // built; a direct caller that skipped that check gets the
        // honest fallback entry kind, and the demux side's codec
        // mapping answers for it.
        let fourcc = sample_entry_fourcc(&state.setup.codec, state.setup.kind);
        debug_assert!(fourcc.is_some(), "runner refused non-h264/aac already");
        let fourcc = fourcc.unwrap_or(b"mp4a");
        let entry_len = 8 + state.setup.config.sample_entry.len();
        let entry_len = u32::try_from(entry_len).unwrap_or(u32::MAX);
        body.extend_from_slice(&entry_len.to_be_bytes());
        body.extend_from_slice(fourcc);
        body.extend_from_slice(state.setup.config.sample_entry.as_slice());
    });
    // stts: run-length durations. Sample i's delta is dts[i+1]-dts[i];
    // the final sample's is the last packet-declared duration (or
    // the previous delta when the container declared none).
    put_box(stbl, b"stts", &mut |body| {
        body.extend_from_slice(&0u32.to_be_bytes());
        let mut runs: Vec<(u32, u32)> = Vec::new();
        for delta in sample_deltas(state) {
            let delta = u32::try_from(delta).unwrap_or(0);
            if let Some(last) = runs.last_mut().filter(|last| last.1 == delta) {
                last.0 += 1;
                continue;
            }
            runs.push((1, delta));
        }
        let count = u32::try_from(runs.len()).unwrap_or(u32::MAX);
        body.extend_from_slice(&count.to_be_bytes());
        for (run, delta) in runs {
            body.extend_from_slice(&run.to_be_bytes());
            body.extend_from_slice(&delta.to_be_bytes());
        }
    });
    // ctts: only when some composition offset is nonzero.
    if rows.iter().any(|r| r.cts != 0) {
        let negative = rows.iter().any(|r| r.cts < 0);
        put_box(stbl, b"ctts", &mut |body| {
            let version_flags: u32 = if negative { 0x0100_0000 } else { 0 };
            body.extend_from_slice(&version_flags.to_be_bytes());
            let mut runs: Vec<(u32, i64)> = Vec::new();
            for r in rows {
                if let Some(last) = runs.last_mut().filter(|last| last.1 == r.cts) {
                    last.0 += 1;
                    continue;
                }
                runs.push((1, r.cts));
            }
            let count = u32::try_from(runs.len()).unwrap_or(u32::MAX);
            body.extend_from_slice(&count.to_be_bytes());
            for (run, offset) in runs {
                body.extend_from_slice(&run.to_be_bytes());
                if negative {
                    body.extend_from_slice(&offset.to_be_bytes());
                } else {
                    body.extend_from_slice(&(offset as u32).to_be_bytes());
                }
            }
        });
    }
    // stss: video only, when not every sample is sync (absence =
    // all-sync, ISO 14496-12 §8.6.2 — the demux side's reading too).
    if state.setup.kind == MediaKind::Video && rows.iter().any(|r| !r.sync) {
        put_box(stbl, b"stss", &mut |body| {
            body.extend_from_slice(&0u32.to_be_bytes());
            let syncs: Vec<u32> = rows
                .iter()
                .enumerate()
                .filter(|(_, r)| r.sync)
                .map(|(i, _)| u32::try_from(i + 1).unwrap_or(1))
                .collect();
            let count = u32::try_from(syncs.len()).unwrap_or(u32::MAX);
            body.extend_from_slice(&count.to_be_bytes());
            for s in syncs {
                body.extend_from_slice(&s.to_be_bytes());
            }
        });
    }
    // stsc: one run — every chunk carries one sample.
    put_box(stbl, b"stsc", &mut |body| {
        body.extend_from_slice(&0u32.to_be_bytes());
        body.extend_from_slice(&1u32.to_be_bytes()); // entry count
        body.extend_from_slice(&1u32.to_be_bytes()); // first_chunk
        body.extend_from_slice(&1u32.to_be_bytes()); // samples_per_chunk
        body.extend_from_slice(&1u32.to_be_bytes()); // sample_description
    });
    // stsz: per-sample sizes.
    put_box(stbl, b"stsz", &mut |body| {
        body.extend_from_slice(&0u32.to_be_bytes());
        body.extend_from_slice(&0u32.to_be_bytes()); // variable sizes
        let count = u32::try_from(rows.len()).unwrap_or(u32::MAX);
        body.extend_from_slice(&count.to_be_bytes());
        for r in rows {
            body.extend_from_slice(&r.size.to_be_bytes());
        }
    });
    // stco: per-sample chunk offsets.
    put_box(stbl, b"stco", &mut |body| {
        body.extend_from_slice(&0u32.to_be_bytes());
        let count = u32::try_from(rows.len()).unwrap_or(u32::MAX);
        body.extend_from_slice(&count.to_be_bytes());
        for r in rows {
            body.extend_from_slice(&r.offset.to_be_bytes());
        }
    });
}

/// Nanoseconds → media ticks, nearest (ties up) — the demux-standard
/// rescale direction.
fn nanos_to_ticks(nanos: u64, timescale: u32) -> u64 {
    if nanos == 0 || timescale == 0 {
        return 0;
    }
    let ticks = (u128::from(nanos) * u128::from(timescale) + 500_000_000) / 1_000_000_000;
    u64::try_from(ticks).unwrap_or(u64::MAX)
}

/// The per-sample `stts` deltas: DTS differences, with the final
/// sample's delta from the last packet-declared duration (or the
/// previous delta when none was declared).
fn sample_deltas(state: &TrackState) -> Vec<u64> {
    let mut out = Vec::with_capacity(state.rows.len());
    for pair in state.rows.windows(2) {
        out.push(pair[1].dts.saturating_sub(pair[0].dts));
    }
    if !state.rows.is_empty() {
        let declared = state.last_duration;
        let prev = out.last().copied().unwrap_or(declared);
        out.push(if declared > 0 { declared } else { prev });
    }
    out
}

/// The track's media duration in ticks (sum of the sample deltas).
fn track_media_duration(state: &TrackState) -> u64 {
    sample_deltas(state)
        .iter()
        .fold(0u64, |acc, delta| acc.saturating_add(*delta))
}

/// The track's duration in movie units (media ticks → 1000ths,
/// nearest).
fn track_movie_duration(state: &TrackState) -> u64 {
    let media = track_media_duration(state);
    if state.setup.timescale == 0 {
        return 0;
    }
    let units = (u128::from(media) * u128::from(MOVIE_TIMESCALE)
        + u128::from(state.setup.timescale) / 2)
        / u128::from(state.setup.timescale);
    u64::try_from(units).unwrap_or(u64::MAX)
}

/// The 16.16 display matrix (identity).
fn put_identity_matrix(body: &mut Vec<u8>) {
    body.extend_from_slice(&0x0001_0000u32.to_be_bytes());
    body.extend_from_slice(&0u32.to_be_bytes());
    body.extend_from_slice(&0u32.to_be_bytes());
    body.extend_from_slice(&0u32.to_be_bytes());
    body.extend_from_slice(&0x0001_0000u32.to_be_bytes());
    body.extend_from_slice(&0u32.to_be_bytes());
    body.extend_from_slice(&0u32.to_be_bytes());
    body.extend_from_slice(&0u32.to_be_bytes());
    body.extend_from_slice(&0x4000_0000u32.to_be_bytes());
}

/// A u64 field as u32, saturating (durations past 49 days in movie
/// units are absurd, and saturating is the crate's overflow policy).
fn put_u32_saturating(body: &mut Vec<u8>, v: u64) {
    body.extend_from_slice(&(v.min(u64::from(u32::MAX)) as u32).to_be_bytes());
}

/// The video dimensions out of a VisualSampleEntry payload (the
/// coded u16 width/height at offsets 24/26); (0, 0) for anything
/// else (audio, or a short/odd payload).
fn video_dimensions(sample_entry: &[u8]) -> (u16, u16) {
    if sample_entry.len() >= 28 {
        let w = u16::from_be_bytes([sample_entry[24], sample_entry[25]]);
        let h = u16::from_be_bytes([sample_entry[26], sample_entry[27]]);
        (w, h)
    } else {
        (0, 0)
    }
}

/// Append one box: `size`, fourcc, then the body built by `fill`.
fn put_box(buf: &mut Vec<u8>, kind: &[u8; 4], fill: &mut dyn FnMut(&mut Vec<u8>)) {
    let start = buf.len();
    buf.extend_from_slice(&0u32.to_be_bytes()); // placeholder
    buf.extend_from_slice(kind);
    fill(buf);
    let size = u32::try_from(buf.len() - start).unwrap_or(u32::MAX);
    buf[start..start + 4].copy_from_slice(&size.to_be_bytes());
}

/// `write_all` with the typed sink fold.
fn write_all(out: &mut Box<dyn WriteSeek>, bytes: &[u8]) -> Result<(), EngineError> {
    out.write_all(bytes).map_err(sink_write_io)
}

/// Absolute `seek` with the typed sink fold.
fn seek_to(out: &mut Box<dyn WriteSeek>, to: u64) -> Result<(), EngineError> {
    out.seek(SeekFrom::Start(to))
        .map(|_| ())
        .map_err(sink_seek_io)
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_types::Packet;
    use media_types::time::{Duration, TimeBase, Timestamp};
    use testkit::{Must, assert_err};

    /// An in-memory `Write + Seek` sink with a shared readable twin
    /// (finalize consumes the muxer, so the bytes must outlive it).
    struct Cursor {
        twin: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
        pos: usize,
    }

    impl Cursor {
        fn new() -> (Cursor, std::sync::Arc<std::sync::Mutex<Vec<u8>>>) {
            let twin = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
            (
                Cursor {
                    twin: std::sync::Arc::clone(&twin),
                    pos: 0,
                },
                twin,
            )
        }
    }

    impl Write for Cursor {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            let mut guard = self.twin.lock().unwrap_or_else(|p| p.into_inner());
            if self.pos + bytes.len() > guard.len() {
                guard.resize(self.pos + bytes.len(), 0);
            }
            guard[self.pos..self.pos + bytes.len()].copy_from_slice(bytes);
            self.pos += bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Seek for Cursor {
        fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
            match to {
                SeekFrom::Start(n) => self.pos = n as usize,
                SeekFrom::Current(n) => {
                    self.pos = usize::try_from(self.pos as i64 + n).unwrap_or(self.pos);
                }
                SeekFrom::End(n) => {
                    let len = self.twin.lock().unwrap_or_else(|p| p.into_inner()).len();
                    self.pos = usize::try_from(len as i64 + n).unwrap_or(self.pos);
                }
            }
            Ok(self.pos as u64)
        }
    }

    fn setup(codec: &str, kind: MediaKind, timescale: u32) -> Mp4TrackSetup {
        Mp4TrackSetup {
            config: Mp4TrackConfig {
                track: TrackId::new(0),
                sample_entry: SharedBytes::from_vec(vec![0u8; 78]),
            },
            codec: codec.to_owned(),
            kind,
            timescale,
        }
    }

    fn packet(track: TrackId, dts: i64, cts: i64, sync: bool) -> Packet {
        Packet {
            track,
            pts: Stamp::Present(Timestamp::new(dts + cts, TimeBase::new(1, 900).must())),
            dts: Stamp::Present(Timestamp::new(dts, TimeBase::new(1, 900).must())),
            duration: Some(Duration::Known(media_types::time::Nanoseconds(1_000_000))),
            data: SharedBytes::from_vec(vec![0xAB; 32]),
            keyframe: sync,
            discontinuity: false,
            generation: media_types::GenerationId::FIRST,
        }
    }

    #[test]
    fn fourcc_mapping_is_the_stream_copy_set() {
        assert_eq!(sample_entry_fourcc("h264", MediaKind::Video), Some(b"avc1"));
        assert_eq!(sample_entry_fourcc("aac", MediaKind::Audio), Some(b"mp4a"));
        // Everything else — including the cross-kind lies — is None.
        assert_eq!(sample_entry_fourcc("h264", MediaKind::Audio), None);
        assert_eq!(sample_entry_fourcc("aac", MediaKind::Video), None);
        for codec in ["hevc", "mjpeg", "pcm_s16le", "opus"] {
            assert_eq!(sample_entry_fourcc(codec, MediaKind::Video), None);
            assert_eq!(sample_entry_fourcc(codec, MediaKind::Audio), None);
        }
    }

    fn muxer_with_twin(
        setups: Vec<Mp4TrackSetup>,
    ) -> (Mp4Remuxer, std::sync::Arc<std::sync::Mutex<Vec<u8>>>) {
        let (cursor, twin) = Cursor::new();
        (Mp4Remuxer::new(Box::new(cursor), setups).must(), twin)
    }

    #[test]
    fn one_track_two_samples_finalizes_a_wellformed_file() {
        let (mut muxer, twin) = muxer_with_twin(vec![setup("h264", MediaKind::Video, 900)]);
        assert_eq!(
            muxer
                .add_packet(0, &packet(TrackId::new(0), 0, 3, true))
                .must(),
            32
        );
        assert_eq!(
            muxer
                .add_packet(0, &packet(TrackId::new(0), 300, 0, false))
                .must(),
            32
        );
        muxer.finalize().must();
        let out = twin.lock().unwrap_or_else(|p| p.into_inner());
        // ftyp is exactly 32 bytes (isom + minor + 4 compat brands);
        // the mdat size field sits at 32..36, its fourcc at 36..40.
        assert_eq!(&out[4..8], b"ftyp");
        assert_eq!(&out[36..40], b"mdat");
        let mdat_size = u32::from_be_bytes([out[32], out[33], out[34], out[35]]);
        assert_eq!(
            mdat_size,
            8 + 64,
            "the patched mdat size covers both samples"
        );
        let moov_at = 32 + 8 + 64;
        assert_eq!(
            &out[moov_at + 4..moov_at + 8],
            b"moov",
            "the moov trails the mdat"
        );
        assert_eq!(
            out.len() - moov_at,
            u32::from_be_bytes([
                out[moov_at],
                out[moov_at + 1],
                out[moov_at + 2],
                out[moov_at + 3],
            ]) as usize,
            "the moov size field is honest"
        );
    }

    #[test]
    fn typed_refusals_of_the_direct_api() {
        let (mut muxer, _twin) = muxer_with_twin(vec![setup("aac", MediaKind::Audio, 44_100)]);
        // Unknown setup ordinal.
        let err = assert_err(
            muxer.add_packet(7, &packet(TrackId::new(0), 0, 0, true)),
            "bad ordinal",
        );
        assert_eq!(err.context.code, "transcode.mp4.track-index");
        // Missing DTS is data, not zero.
        let mut missing = packet(TrackId::new(0), 0, 0, true);
        missing.dts = Stamp::Missing;
        let err = assert_err(muxer.add_packet(0, &missing), "no dts");
        assert_eq!(err.context.code, "transcode.mp4.stamp-missing");
        // Backwards DTS.
        muxer
            .add_packet(0, &packet(TrackId::new(0), 500, 0, true))
            .must();
        let err = assert_err(
            muxer.add_packet(0, &packet(TrackId::new(0), 100, 0, true)),
            "dts went backwards",
        );
        assert_eq!(err.context.code, "transcode.mp4.dts-non-monotonic");
    }

    #[test]
    fn the_sample_table_ceiling_is_typed() {
        // The guard arithmetic, driven directly (the same check
        // `add_packet` makes — a full 1 Mi-row table is pre-filled, no
        // sample bytes materialized): the next add refuses typed.
        let (mut muxer, _twin) = muxer_with_twin(vec![setup("aac", MediaKind::Audio, 44_100)]);
        let state = &mut muxer.tracks[0];
        state.origin = Some(0); // rows are already anchored at tick 0
        state.rows = (0..MAX_REMUX_SAMPLES)
            .map(|i| SampleRow {
                offset: 40,
                size: 0,
                dts: u64::try_from(i).unwrap_or(0),
                cts: 0,
                sync: true,
            })
            .collect();
        let err = assert_err(
            muxer.add_packet(
                0,
                &packet(TrackId::new(0), MAX_REMUX_SAMPLES as i64, 0, true),
            ),
            "over the sample ceiling",
        );
        assert_eq!(err.context.code, "transcode.mp4.samples-over-budget");
        assert_eq!(MAX_REMUX_SAMPLES, 1024 * 1024);
    }
}
