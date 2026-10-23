//! # bench-vs-vlc — the Lumen half of the differential-vs-VLC harness
//! (card 3.E3, doc 10 §6.6)
//!
//! Runs a corpus fixture through the **real** Lumen decode path — the
//! same composition the CLI's `decode-bench` uses (canonicalize →
//! `input` access → `media-probe` → `AviDemuxer` → `codecs`
//! decoders) — and digests the **first PCM audio track's** decoded
//! PCM with MD5, so the Python orchestrator (`run.py`) can compare it
//! against the pinned external VLC 3.0.23 oracle's dump of the same
//! track.
//!
//! ## Why a test-tool crate and not a CLI flag
//!
//! `apps/cli` is not this card's scope; the honest seam is this
//! standalone workspace crate whose **integration test**
//! (`tests/lumen_pcm.rs`) decodes and digests under `cargo test`
//! (tests may depend on workspace crates; the card's recorded
//! decision). The Python harness drives it:
//!
//! ```text
//! LUMEN_DIFF_FIXTURES=<tsv: rel<TAB>path>  LUMEN_DIFF_OUT=<json>
//! cargo test --manifest-path tools/bench-vs-vlc/Cargo.toml \
//!     --test lumen_pcm -- --ignored --nocapture
//! ```
//!
//! ## Comparison semantics (doc 10 §6.6, "Decoded audio")
//!
//! - Track rule: **first audio track** (VLC's default-track choice —
//!   `[VERIFY: VLC default audio track == first, pending the real
//!   oracle run]`).
//! - Format rule: the container's own PCM family, byte-exact. The
//!   AVI/WAVE PCM wire is little-endian by definition and Lumen's
//!   `PcmDecoder` normalizes to native-endian samples, so on a
//!   little-endian host the decoded bytes for `pcm_u8`/`pcm_s16le`/
//!   `pcm_f32le` are the passthrough bytes — exactly what VLC's
//!   `afile` output module dumps for its `u8`/`s16`/`float32`
//!   formats (native-endian). The host's endianness is recorded in
//!   every JSON envelope; the orchestrator refuses comparability on
//!   big-endian hosts.
//! - MD5: in-repo RFC 1321 implementation ([`md5`]) — zero new
//!   dependencies (AGENTS.md §3.2).
//!
//! ## Damage policy (mirrors the engine)
//!
//! Per-frame damage (corrupt packet, decoder bitstream error) is
//! **concealed** and counted — same rule as `apps/cli`'s bench and
//! `playback-engine`'s pipeline (`per_frame_damage` + the
//! 1000-consecutive budget). A concealed frame means the digest is
//! over a damaged decode; the JSON row says so (`concealed > 0`), and
//! the orchestrator labels such rows non-clean. Demux `Recovered`
//! events are counted too, split two ways:
//!
//! * `demux_recovered` — the count of `Recovered` events. On clean
//!   AVIs whose video codec the v0 filter refuses (e.g. mpeg4), the
//!   walk's start emits one `Recovered` per filtered stream — a
//!   *decision record* (`skipped == 0`, reason
//!   `unsupported-video-handler`, the D-018 filter), not damage. On
//!   this build the filtered streams' `idx1` entries additionally
//!   surface `idx1-entry-unknown-stream` (16 B per entry) — seek-table
//!   drops only; the W5-14 branch (pending merge at time of writing)
//!   silences exactly those for declared-but-filtered streams.
//! * `demux_skip_bytes` — the sum of the events' `skipped` fields.
//!   Zero for pure decision records; nonzero means the walk either
//!   resynchronized past container bytes or dropped index entries —
//!   the report flags such rows and the run notes record what was
//!   independently verified.
//!
//! v1 compares **clean** corpus files only; this machinery exists so
//! a dirty file can never silently produce an "ok" digest.

pub mod md5;

use std::path::{Path, PathBuf};

use codec_api::{DecodeOutcome, Decoder, DecoderConfig, FrameSink, ThreadBudget};
use codecs::audio::pcm::PcmDecoder;
use demux::avi::AviDemuxer;
use demux::avi::header::StreamFormat;
use input::FileAccess;
use media_probe::{ProbeConfig, ProbeRegistry};
use media_types::error::DecodeCause;
use media_types::frame::{AudioLayout, SampleFormat};
use media_types::{AudioFormat, EngineError, ErrorKind, Frame, GenerationId, MediaKind};
use plugin_api::{Access, DemuxEvent, Demuxer};

/// The maximum run of consecutive concealed per-frame damage before
/// the digest gives up (mirrors the engine's concealment budget; a
/// stream whose every packet fails is broken, not comparable).
pub const CONCEALMENT_BUDGET: u32 = 1000;

/// The host's byte order, as the JSON envelopes spell it.
pub fn host_endian() -> &'static str {
    if cfg!(target_endian = "little") {
        "little"
    } else {
        "big"
    }
}

/// Whether an error is per-frame damage — the class the engine
/// conceals instead of tearing down (mirrored from the CLI bench,
/// which mirrors `playback-engine`; the engine's own is
/// crate-private).
fn per_frame_damage(error: &EngineError) -> bool {
    matches!(
        &error.kind,
        ErrorKind::Corrupt { .. }
            | ErrorKind::Decoder {
                cause: DecodeCause::Bitstream
            }
    )
}

/// The digest-accumulating frame sink: every accepted audio frame's
/// PCM bytes feed the MD5 state, in decode order.
pub struct DigestSink {
    digest: md5::Md5,
    /// PCM bytes absorbed.
    pub bytes: u64,
    /// Audio frames accepted.
    pub frames: u64,
    /// Non-audio frames accepted (wiring error signal — this harness
    /// only wires the audio limb, so anything else is a bug).
    pub other_kinds: u64,
    /// The first frame's format (the stream's; a mid-stream change is
    /// a decode anomaly the orchestrator should see).
    pub format: Option<AudioFormat>,
    /// Formats seen after the first (empty in a sane stream).
    pub format_changes: u32,
}

impl DigestSink {
    /// A fresh sink over the empty message.
    pub fn new() -> DigestSink {
        DigestSink {
            digest: md5::Md5::new(),
            bytes: 0,
            frames: 0,
            other_kinds: 0,
            format: None,
            format_changes: 0,
        }
    }

    /// The digest of everything accepted so far, lowercase hex.
    pub fn digest_hex(&self) -> String {
        // Md5::finalize consumes self; clone the streaming state.
        let clone = self.digest.clone();
        clone.finalize_hex()
    }
}

impl Default for DigestSink {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameSink for DigestSink {
    fn accept(&mut self, frame: Frame) -> Result<(), EngineError> {
        match frame {
            Frame::Audio(audio) => {
                if let Some(seen) = self.format {
                    if seen != audio.format {
                        self.format_changes = self.format_changes.saturating_add(1);
                    }
                } else {
                    self.format = Some(audio.format);
                }
                self.digest.update(audio.samples.as_slice());
                self.bytes += audio.samples.as_slice().len() as u64;
                self.frames += 1;
                Ok(())
            }
            Frame::Video(_) | Frame::Subtitle(_) => {
                self.other_kinds += 1;
                Ok(())
            }
        }
    }
}

/// What one fixture's digest produced (the ok half of a batch row).
#[derive(Debug, Clone)]
pub struct PcmDigestReport {
    /// MD5 of the decoded PCM stream, lowercase hex.
    pub md5_hex: String,
    /// Decoded PCM bytes absorbed.
    pub pcm_bytes: u64,
    /// Audio frames accepted.
    pub frames: u64,
    /// Audio packets fed to the decoder.
    pub packets: u64,
    /// Per-frame damage concealed (nonzero ⇒ damaged decode).
    pub concealed: u64,
    /// Demux `Recovered` events (includes the v0 codec filter's
    /// decision records — `skipped == 0` — for refused video streams;
    /// see the module docs).
    pub demux_recovered: u64,
    /// Bytes the demuxer skipped while recovering (the sum of the
    /// `Recovered` events' `skipped`): 0 for pure decision records,
    /// nonzero ⇒ the digest walked past container damage.
    pub demux_skip_bytes: u64,
    /// The registry PCM descriptor (`pcm_s16le`, …).
    pub codec: String,
    /// Channel count.
    pub channels: u32,
    /// Sample rate.
    pub rate: u32,
    /// The emitted sample format (`S16`, `F32`, …).
    pub sample: String,
    /// The emitted layout (`interleaved`/`planar`).
    pub layout: String,
    /// The digested track's demuxer stream ordinal.
    pub track_handle: u32,
    /// Mid-stream format changes observed (0 in a sane stream).
    pub format_changes: u32,
}

/// Percent-encode a path for a `file:` URI (the CLI open path's rule:
/// RFC 3986 unreserved + `/` kept, everything else escaped).
fn encode_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for byte in path.bytes() {
        let keep = byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'/');
        if keep {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// The sample-format name as the JSON/report spells it.
fn sample_name(format: AudioFormat) -> String {
    let name = match format.sample {
        SampleFormat::U8 => "u8",
        SampleFormat::S16 => "s16",
        SampleFormat::S24 => "s24",
        SampleFormat::S32 => "s32",
        SampleFormat::F32 => "f32",
        SampleFormat::F64 => "f64",
    };
    name.to_string()
}

/// Digest one AVI fixture's first PCM audio track end to end.
///
/// # Errors
/// A human-readable, stage-tagged error string (`[input] …`,
/// `[probe] …`, `[demux] …`, `[decode] …`) — the batch JSON carries
/// it verbatim; nothing here panics (dirty-world rule D-3: the
/// adversarial corpus is an expected input).
pub fn digest_avi_first_pcm_audio(path: &Path) -> Result<PcmDigestReport, String> {
    // 1. Canonicalize + build the `file:` URI (the player's open
    //    path; symlink-safe by construction).
    let canonical = std::fs::canonicalize(path)
        .map_err(|err| format!("[input] cannot resolve {}: {err}", path.display()))?;
    if !canonical.is_file() {
        return Err(format!("[input] not a regular file: {}", path.display()));
    }
    let text = canonical
        .to_str()
        .ok_or_else(|| format!("[input] the path is not valid UTF-8: {}", path.display()))?;
    let uri = media_types::SourceUri::parse(&format!("file://{}", encode_path(text)))
        .map_err(|err| format!("[input] encoded file URI rejected: {err}"))?;

    // 2. Access (the 2.A1 stack).
    let access = FileAccess::new();
    let mut stream = access.open(&uri).map_err(|err| format!("[input] {err}"))?;

    // 3. Probe (the 2.A2 bounded window; the honest container truth).
    let registry = ProbeRegistry::with_builtins();
    let config = ProbeConfig::default();
    let report = media_probe::probe_stream(&registry, &mut *stream, &config)
        .map_err(|err| format!("[probe] {err}"))?;
    if report.format.as_str() != "avi" {
        return Err(format!(
            "[probe] format `{}` has no demuxer in this build (first vertical: AVI, D-018)",
            report.format.as_str()
        ));
    }
    digest_avi_first_pcm_audio_from(stream)
}

/// The digest pipeline over an already-opened, already-probed AVI
/// stream (the in-memory seam the synthetic-PCM tests use).
///
/// # Errors
/// Stage-tagged error strings, same contract as
/// [`digest_avi_first_pcm_audio`].
pub fn digest_avi_first_pcm_audio_from(
    stream: Box<dyn plugin_api::SourceStream>,
) -> Result<PcmDigestReport, String> {
    // 1. Demuxer + track selection (first audio track whose strf is
    //    PCM — the VLC default-track mirror).
    let mut demuxer = AviDemuxer::new(stream).map_err(|err| format!("[demux] {err}"))?;
    let tracks = demuxer.start().map_err(|err| format!("[demux] {err}"))?;
    let mut chosen = None;
    for track in &tracks {
        if track.kind == MediaKind::Audio {
            match demuxer.stream_format(track.id) {
                Some(StreamFormat::Pcm {
                    sample,
                    channels,
                    samples_per_sec,
                    ..
                }) => {
                    // The registry descriptor the strf declares (the
                    // demuxer's own mapping — never a local copy).
                    let codec = sample.descriptor().to_string();
                    chosen = Some((track.clone(), codec, u32::from(channels), samples_per_sec));
                    break;
                }
                other => {
                    return Err(format!(
                        "[demux] first audio track is not PCM (strf says {other:?}) — not a \
                         lossless-comparable path in this build"
                    ));
                }
            }
        }
    }
    let Some((track, codec, channels, rate)) = chosen else {
        return Err("[demux] no audio track (nothing to digest)".to_string());
    };

    // 2. Decoder behind the real contract (the with_format seam).
    let mut decoder =
        PcmDecoder::with_format(channels, rate).map_err(|err| format!("[decode] {err}"))?;
    let config = DecoderConfig::new(
        track.codec.clone(),
        GenerationId::FIRST,
        ThreadBudget::default(),
        None,
        None,
    );
    decoder
        .configure(&config)
        .map_err(|err| format!("[decode] {err}"))?;

    // 3. The digest loop (concealment policy mirrored from the bench).
    let mut sink = DigestSink::new();
    let mut packets: u64 = 0;
    let mut concealed: u64 = 0;
    let mut demux_recovered: u64 = 0;
    let mut demux_skip_bytes: u64 = 0;
    let mut consecutive: u32 = 0;
    loop {
        match demuxer.next_event() {
            Ok(DemuxEvent::Packet(packet)) => {
                if packet.track == track.id {
                    packets += 1;
                    match decoder.decode(packet, &mut sink) {
                        Ok(DecodeOutcome::Emitted { .. }) => consecutive = 0,
                        Ok(DecodeOutcome::Recovered { .. }) => {
                            consecutive = consecutive.saturating_add(1);
                        }
                        Ok(DecodeOutcome::NeedMoreInput | DecodeOutcome::Drained) => {}
                        Err(error) if per_frame_damage(&error) => {
                            concealed += 1;
                            consecutive = consecutive.saturating_add(1);
                            if consecutive >= CONCEALMENT_BUDGET {
                                return Err(format!(
                                    "[decode] {CONCEALMENT_BUDGET} consecutive damaged packets — \
                                     concealment budget exhausted: {error}"
                                ));
                            }
                        }
                        Err(error) => return Err(format!("[decode] {error}")),
                    }
                }
                // Packets of other tracks are dropped (the track rule
                // above said which track is compared).
            }
            Ok(DemuxEvent::Recovered { skipped, .. }) => {
                demux_recovered += 1;
                demux_skip_bytes = demux_skip_bytes.saturating_add(skipped);
            }
            Ok(DemuxEvent::Discontinuity) => {}
            Ok(DemuxEvent::EndOfStream) => break,
            Err(error) => return Err(format!("[demux] mid-walk fatal: {error}")),
        }
    }
    // The flush outcome's frames (a stateless decoder answers
    // `Drained`) already rode the sink — the counters below are the
    // truth; the outcome itself carries nothing the digest needs.
    match decoder.flush(&mut sink) {
        Ok(_) => {}
        Err(error) => return Err(format!("[decode] flush: {error}")),
    }

    let format = sink.format.ok_or_else(|| {
        "[decode] the decoder emitted no audio frames (empty stream?)".to_string()
    })?;
    Ok(PcmDigestReport {
        md5_hex: sink.digest_hex(),
        pcm_bytes: sink.bytes,
        frames: sink.frames,
        packets,
        concealed,
        demux_recovered,
        demux_skip_bytes,
        codec,
        channels,
        rate,
        sample: sample_name(format),
        layout: match format.layout {
            AudioLayout::Interleaved => "interleaved".to_string(),
            AudioLayout::Planar => "planar".to_string(),
        },
        track_handle: track.id.handle(),
        format_changes: sink.format_changes,
    })
}

/// One batch row: a fixture's outcome.
#[derive(Debug, Clone)]
pub enum BatchOutcome {
    /// The digest (and its accounting).
    Ok {
        rel: String,
        report: PcmDigestReport,
    },
    /// The typed refusal (stage-tagged message).
    Error { rel: String, error: String },
}

/// Digest a batch of fixtures (`rel` = corpus-relative id, `path` =
/// absolute file).
pub fn digest_batch(fixtures: &[(String, PathBuf)]) -> Vec<BatchOutcome> {
    fixtures
        .iter()
        .map(|(rel, path)| match digest_avi_first_pcm_audio(path) {
            Ok(report) => BatchOutcome::Ok {
                rel: rel.clone(),
                report,
            },
            Err(error) => BatchOutcome::Error {
                rel: rel.clone(),
                error,
            },
        })
        .collect()
}

/// Escape one string for JSON (quotes, backslash, control chars).
fn json_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Render the batch as the harness's JSON envelope (schema
/// `bench-vs-vlc/lumen-pcm-digests/1`).
pub fn render_batch_json(outcomes: &[BatchOutcome]) -> String {
    let mut rows = String::new();
    for (index, outcome) in outcomes.iter().enumerate() {
        if index > 0 {
            rows.push_str(",\n");
        }
        match outcome {
            BatchOutcome::Ok { rel, report } => {
                rows.push_str(&format!(
                    "  {{\"rel\": \"{}\", \"status\": \"ok\", \"md5\": \"{}\", \"pcm_bytes\": {}, \
                     \"frames\": {}, \"packets\": {}, \"codec\": \"{}\", \"channels\": {}, \
                     \"rate\": {}, \"sample\": \"{}\", \"layout\": \"{}\", \"track\": {}, \
                     \"concealed\": {}, \"demux_recovered\": {}, \"demux_skip_bytes\": {}, \
                     \"format_changes\": {}}}",
                    json_escape(rel),
                    report.md5_hex,
                    report.pcm_bytes,
                    report.frames,
                    report.packets,
                    json_escape(&report.codec),
                    report.channels,
                    report.rate,
                    report.sample,
                    report.layout,
                    report.track_handle,
                    report.concealed,
                    report.demux_recovered,
                    report.demux_skip_bytes,
                    report.format_changes,
                ));
            }
            BatchOutcome::Error { rel, error } => {
                rows.push_str(&format!(
                    "  {{\"rel\": \"{}\", \"status\": \"error\", \"error\": \"{}\"}}",
                    json_escape(rel),
                    json_escape(error)
                ));
            }
        }
    }
    format!(
        "{{\n  \"schema\": \"bench-vs-vlc/lumen-pcm-digests/1\",\n  \"host_endian\": \"{}\",\n  \
         \"results\": [\n{}\n  ]\n}}\n",
        host_endian(),
        rows
    )
}

/// Parse the harness's fixture list (one `rel<TAB>path` per line;
/// `#`-prefixed lines and blanks ignored).
///
/// # Errors
/// A line without a tab, or a duplicated `rel`.
pub fn parse_fixture_tsv(text: &str) -> Result<Vec<(String, PathBuf)>, String> {
    let mut fixtures = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (number, line) in text.lines().enumerate() {
        let line = line.trim_end_matches(['\r']);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((rel, path)) = line.split_once('\t') else {
            return Err(format!(
                "fixture list line {}: expected rel<TAB>path",
                number + 1
            ));
        };
        if !seen.insert(rel.to_string()) {
            return Err(format!(
                "fixture list line {}: duplicate rel {rel}",
                number + 1
            ));
        }
        fixtures.push((rel.to_string(), PathBuf::from(path)));
    }
    Ok(fixtures)
}
