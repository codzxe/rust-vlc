//! Typed error constructors at stage [`PipelineStage::Transcode`] —
//! the crate's whole mint vocabulary, one table, tested in one place
//! (the `input` crate's error-module precedent).
//!
//! The *taxonomy* is media-types' (never redefined here): what this
//! module owns are the stable dotted context codes and the kind each
//! one carries. Codes are part of the compatibility surface once a
//! release ships, so additions are additive-only.

use media_types::error::{
    AccessDenied, CorruptDetail, ErrorContext, ErrorKind, FormatInfo, LimitKind,
};
use media_types::{EngineError, PipelineStage};

/// Mint a transcode-stage error with `kind` and `code`.
fn mint(kind: ErrorKind, code: &'static str) -> EngineError {
    EngineError::new(
        PipelineStage::Transcode,
        kind,
        ErrorContext::new(code),
    )
}

/// The sink stage requires a decode stage (graph shape: WavPcm without
/// a decoder factory).
pub fn graph_decode_required() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "transcode.graph.decode-required",
    )
}

/// A stream-copy sink was given a decode stage (graph shape: copy
/// targets consume demuxed packets, never decoded frames).
pub fn graph_decode_forbidden() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "transcode.graph.decode-forbidden",
    )
}

/// A WAV stream-copy graph without source PCM facts (the `fmt `-level
/// facts the RIFF writer needs before the first packet).
pub fn graph_copy_facts_missing() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "transcode.graph.copy-facts-missing",
    )
}

/// A filter stage was requested; v1 implements none (honest-capability
/// rule — the model carries the stage, validation refuses it).
pub fn filter_unimplemented() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "transcode.graph.filter-unimplemented",
    )
}

/// More filter stages than the configured budget.
pub fn filter_stages_over_budget(requested: usize, budget: usize) -> EngineError {
    mint(
        ErrorKind::ResourceLimit {
            limit: LimitKind::QueueSize,
            budget: budget as u64,
            attempted: requested as u64,
        },
        "transcode.graph.filter-stages-over-budget",
    )
}

/// A job limit of zero where zero is unusable.
pub fn limit_zero() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "transcode.limits.zero",
    )
}

/// A job limit above its hard ceiling (bounded memory is a contract,
/// not a suggestion — MP §12; the input crate's capacity precedent).
pub fn limit_over_budget(requested: usize, budget: usize) -> EngineError {
    mint(
        ErrorKind::ResourceLimit {
            limit: LimitKind::Memory,
            budget: budget as u64,
            attempted: requested as u64,
        },
        "transcode.limits.over-budget",
    )
}

/// The demuxer enumerated more tracks than the job's track budget.
pub fn tracks_over_budget(found: usize, budget: usize) -> EngineError {
    mint(
        ErrorKind::ResourceLimit {
            limit: LimitKind::QueueSize,
            budget: budget as u64,
            attempted: found as u64,
        },
        "transcode.source.tracks-over-budget",
    )
}

/// `TrackSelection::DefaultAudio` found no audio track.
pub fn audio_track_missing() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "transcode.source.audio-track-missing",
    )
}

/// `TrackSelection::DefaultAudio` found more than one audio track —
/// the caller must name one.
pub fn audio_track_ambiguous(found: usize) -> EngineError {
    let _ = found;
    mint(
        ErrorKind::Unsupported { format: None },
        "transcode.source.audio-track-ambiguous",
    )
}

/// The explicitly selected track does not exist.
pub fn track_not_found() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "transcode.source.track-not-found",
    )
}

/// The selected track is not audio (WAV targets transcribe audio).
pub fn track_not_audio() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "transcode.source.track-not-audio",
    )
}

/// A stream-copy target met a track whose codec is not unpacked PCM.
pub fn copy_not_pcm(descriptor: &str) -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some(descriptor.to_owned()),
                sniffed: Vec::new(),
            }),
        },
        "transcode.copy.not-pcm",
    )
}

/// A stream-copy target met little/big-endian mismatched PCM: the copy
/// path writes payload bytes verbatim, so a byte-swapped source needs
/// the decode path instead.
pub fn copy_byte_order(descriptor: &str) -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some(descriptor.to_owned()),
                sniffed: Vec::new(),
            }),
        },
        "transcode.copy.byte-order",
    )
}

/// The caller's PCM facts disagree with the selected track's codec
/// descriptor (the one cross-check the L0 `Track` vocabulary allows —
/// it carries the descriptor but no codec parameters).
pub fn copy_facts_mismatch(descriptor: &str, facts: &str) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure { offset: None },
        },
        "transcode.copy.facts-mismatch",
    )
    .with_source(FactsMismatch {
        descriptor: descriptor.to_owned(),
        facts: facts.to_owned(),
    })
}

/// The tiny source-chaining leaf for [`copy_facts_mismatch`] (the
/// plugin-api `DemuxSourceInfo` precedent).
#[derive(Debug)]
struct FactsMismatch {
    descriptor: String,
    facts: String,
}

impl std::fmt::Display for FactsMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "track codec `{}` vs facts `{}`", self.descriptor, self.facts)
    }
}
impl std::error::Error for FactsMismatch {}

/// The WAV encoder was configured for a codec descriptor outside the
/// PCM family it writes (`pcm_u8` … `pcm_f64le`, doc 04 §3.1).
pub fn wav_codec_not_pcm(descriptor: &str) -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some(descriptor.to_owned()),
                sniffed: Vec::new(),
            }),
        },
        "transcode.wav.codec-not-pcm",
    )
}

/// The decode path delivered a non-audio frame to an audio encoder.
pub fn frame_not_audio() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "transcode.wav.frame-not-audio",
    )
}

/// No exact conversion exists from the decoded sample format to the
/// WAV target (v1 converts identity, planar→interleaved, and the
/// lossless integer/float promotions — everything else is refused,
/// never silently quantized).
pub fn wav_conversion_unsupported(from: &str, to: &str) -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some(format!("{from}->{to}")),
                sniffed: Vec::new(),
            }),
        },
        "transcode.wav.sample-conversion",
    )
}

/// The decoded format changed mid-stream (channels or rate) — the
/// documented v0 gap (the WAV demuxer refuses the same shape).
pub fn wav_format_change() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "transcode.wav.format-change",
    )
}

/// A payload handed to the WAV writer was not a whole number of
/// sample frames.
pub fn wav_partial_frame() -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure { offset: None },
        },
        "transcode.wav.partial-frame",
    )
}

/// The WAV facts carry a channel count outside 1..=MAX_CHANNELS.
pub fn wav_channels_out_of_range(channels: u32) -> EngineError {
    let _ = channels;
    mint(
        ErrorKind::Unsupported { format: None },
        "transcode.wav.channels-out-of-range",
    )
}

/// The WAV facts carry a zero sample rate.
pub fn wav_rate_zero() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "transcode.wav.rate-zero",
    )
}

/// `rate × block_align` does not fit the fmt chunk's 32-bit byte-rate
/// field.
pub fn wav_byte_rate_overflow() -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure { offset: None },
        },
        "transcode.wav.byte-rate-overflow",
    )
}

/// The output grew past what a classic RIFF/WAVE (not RF64) file can
/// address — the v1 ceiling, typed rather than wrapped.
pub fn wav_size_overflow(attempted: u64) -> EngineError {
    mint(
        ErrorKind::ResourceLimit {
            limit: LimitKind::Memory,
            budget: u32::MAX as u64,
            attempted,
        },
        "transcode.wav.size-overflow",
    )
}

/// A sink write failed (folded with the OS error as the chain root —
/// the input crate's `read_io` folding precedent).
pub fn sink_write_io(err: std::io::Error) -> EngineError {
    mint(
        ErrorKind::Inaccessible {
            reason: AccessDenied::PermissionDenied,
        },
        "transcode.sink.write-io",
    )
    .with_source(err)
}

/// A sink seek failed (header patch time).
pub fn sink_seek_io(err: std::io::Error) -> EngineError {
    mint(
        ErrorKind::Inaccessible {
            reason: AccessDenied::PermissionDenied,
        },
        "transcode.sink.seek-io",
    )
    .with_source(err)
}

/// A worker thread could not be spawned.
pub fn worker_spawn_io(err: std::io::Error) -> EngineError {
    mint(
        ErrorKind::Inaccessible {
            reason: AccessDenied::PermissionDenied,
        },
        "transcode.job.worker-spawn-io",
    )
    .with_source(err)
}

/// A worker thread ended without a terminal state — a panic in a
/// caller-provided stage (the decode contract treats panics as
/// decoder faults and discards the instance; the job ends honestly).
pub fn worker_panic() -> EngineError {
    mint(
        ErrorKind::Decoder {
            cause: media_types::error::DecodeCause::BadState,
        },
        "transcode.job.worker-panic",
    )
}

/// The decoder's flush did not reach `Drained` inside the flush
/// budget (the playback engine's bounded-drain doctrine).
pub fn flush_undrained() -> EngineError {
    mint(
        ErrorKind::Decoder {
            cause: media_types::error::DecodeCause::BadState,
        },
        "transcode.decode.flush-undrained",
    )
}

/// An internal sink state machine violation (a typed stand-in for an
/// impossible state — panics are contract violations, errors are
/// values).
pub fn sink_bad_state() -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure { offset: None },
        },
        "transcode.sink.bad-state",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every constructor mints stage `Transcode` with the kind and
    /// code it claims — the whole vocabulary, one table (the input
    /// crate's precedent).
    #[test]
    fn constructors_mint_stage_transcode_kinds_and_codes() {
        let cases: [(EngineError, ErrorKind, &str); 24] = [
            (
                graph_decode_required(),
                ErrorKind::Unsupported { format: None },
                "transcode.graph.decode-required",
            ),
            (
                graph_decode_forbidden(),
                ErrorKind::Unsupported { format: None },
                "transcode.graph.decode-forbidden",
            ),
            (
                graph_copy_facts_missing(),
                ErrorKind::Unsupported { format: None },
                "transcode.graph.copy-facts-missing",
            ),
            (
                filter_unimplemented(),
                ErrorKind::Unsupported { format: None },
                "transcode.graph.filter-unimplemented",
            ),
            (
                filter_stages_over_budget(4, 2),
                ErrorKind::ResourceLimit {
                    limit: LimitKind::QueueSize,
                    budget: 2,
                    attempted: 4,
                },
                "transcode.graph.filter-stages-over-budget",
            ),
            (
                limit_zero(),
                ErrorKind::Unsupported { format: None },
                "transcode.limits.zero",
            ),
            (
                limit_over_budget(9, 8),
                ErrorKind::ResourceLimit {
                    limit: LimitKind::Memory,
                    budget: 8,
                    attempted: 9,
                },
                "transcode.limits.over-budget",
            ),
            (
                tracks_over_budget(300, 256),
                ErrorKind::ResourceLimit {
                    limit: LimitKind::QueueSize,
                    budget: 256,
                    attempted: 300,
                },
                "transcode.source.tracks-over-budget",
            ),
            (
                audio_track_missing(),
                ErrorKind::Unsupported { format: None },
                "transcode.source.audio-track-missing",
            ),
            (
                audio_track_ambiguous(2),
                ErrorKind::Unsupported { format: None },
                "transcode.source.audio-track-ambiguous",
            ),
            (
                track_not_found(),
                ErrorKind::Unsupported { format: None },
                "transcode.source.track-not-found",
            ),
            (
                track_not_audio(),
                ErrorKind::Unsupported { format: None },
                "transcode.source.track-not-audio",
            ),
            (
                copy_not_pcm("mjpeg"),
                ErrorKind::Unsupported {
                    format: Some(FormatInfo {
                        name: Some("mjpeg".to_owned()),
                        sniffed: Vec::new(),
                    }),
                },
                "transcode.copy.not-pcm",
            ),
            (
                copy_byte_order("pcm_s16be"),
                ErrorKind::Unsupported {
                    format: Some(FormatInfo {
                        name: Some("pcm_s16be".to_owned()),
                        sniffed: Vec::new(),
                    }),
                },
                "transcode.copy.byte-order",
            ),
            (
                copy_facts_mismatch("pcm_s16le", "pcm_s24le"),
                ErrorKind::Corrupt {
                    detail: CorruptDetail::InvalidStructure { offset: None },
                },
                "transcode.copy.facts-mismatch",
            ),
            (
                wav_codec_not_pcm("vorbis"),
                ErrorKind::Unsupported {
                    format: Some(FormatInfo {
                        name: Some("vorbis".to_owned()),
                        sniffed: Vec::new(),
                    }),
                },
                "transcode.wav.codec-not-pcm",
            ),
            (
                frame_not_audio(),
                ErrorKind::Unsupported { format: None },
                "transcode.wav.frame-not-audio",
            ),
            (
                wav_conversion_unsupported("pcm_s16le", "pcm_f32le"),
                ErrorKind::Unsupported {
                    format: Some(FormatInfo {
                        name: Some("pcm_s16le->pcm_f32le".to_owned()),
                        sniffed: Vec::new(),
                    }),
                },
                "transcode.wav.sample-conversion",
            ),
            (
                wav_format_change(),
                ErrorKind::Unsupported { format: None },
                "transcode.wav.format-change",
            ),
            (
                wav_partial_frame(),
                ErrorKind::Corrupt {
                    detail: CorruptDetail::InvalidStructure { offset: None },
                },
                "transcode.wav.partial-frame",
            ),
            (
                wav_rate_zero(),
                ErrorKind::Unsupported { format: None },
                "transcode.wav.rate-zero",
            ),
            (
                wav_byte_rate_overflow(),
                ErrorKind::Corrupt {
                    detail: CorruptDetail::InvalidStructure { offset: None },
                },
                "transcode.wav.byte-rate-overflow",
            ),
            (
                wav_size_overflow(u32::MAX as u64 + 1),
                ErrorKind::ResourceLimit {
                    limit: LimitKind::Memory,
                    budget: u32::MAX as u64,
                    attempted: u32::MAX as u64 + 1,
                },
                "transcode.wav.size-overflow",
            ),
            (
                sink_bad_state(),
                ErrorKind::Corrupt {
                    detail: CorruptDetail::InvalidStructure { offset: None },
                },
                "transcode.sink.bad-state",
            ),
        ];
        for (error, kind, code) in cases {
            assert_eq!(error.stage, PipelineStage::Transcode, "{code}");
            assert_eq!(error.kind, kind, "{code}");
            assert_eq!(error.context.code, code);
        }
    }

    /// The io-folding constructors keep the OS error on the chain and
    /// the panic/flush constructors carry their decoder-fault kinds.
    #[test]
    fn io_and_fault_constructors_carry_sources_and_kinds() {
        let write = sink_write_io(std::io::Error::other("no space"));
        assert_eq!(write.context.code, "transcode.sink.write-io");
        assert!(std::error::Error::source(&write).is_some(), "io root kept");

        let seek = sink_seek_io(std::io::Error::other("espipe"));
        assert_eq!(seek.context.code, "transcode.sink.seek-io");
        assert!(std::error::Error::source(&seek).is_some());

        let spawn_err = worker_spawn_io(std::io::Error::other("try again"));
        assert_eq!(spawn_err.context.code, "transcode.job.worker-spawn-io");
        assert!(std::error::Error::source(&spawn_err).is_some());

        let panic_err = worker_panic();
        assert_eq!(
            panic_err.kind,
            ErrorKind::Decoder {
                cause: media_types::error::DecodeCause::BadState
            }
        );
        assert_eq!(panic_err.context.code, "transcode.job.worker-panic");

        let flush = flush_undrained();
        assert_eq!(
            flush.kind,
            ErrorKind::Decoder {
                cause: media_types::error::DecodeCause::BadState
            }
        );
        assert_eq!(flush.context.code, "transcode.decode.flush-undrained");

        // The remaining channel-count constructor (not in the table
        // above to keep it at the array size literal).
        let channels = wav_channels_out_of_range(0);
        assert_eq!(channels.context.code, "transcode.wav.channels-out-of-range");
        assert_eq!(
            channels.kind,
            ErrorKind::Unsupported { format: None }
        );

        // The mismatch leaf renders both sides.
        let mismatch = copy_facts_mismatch("pcm_s16le", "pcm_u8");
        let rendered = match std::error::Error::source(&mismatch) {
            Some(src) => src.to_string(),
            None => unreachable!("leaf is on the chain"),
        };
        assert!(rendered.contains("pcm_s16le"), "{rendered}");
        assert!(rendered.contains("pcm_u8"), "{rendered}");
    }
}
