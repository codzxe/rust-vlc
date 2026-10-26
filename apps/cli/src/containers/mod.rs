//! THE container-host registry (6.OPW-PREP; the seam design's §4.4 —
//! `docs/plans/open-path-wiring.md`): the open path's single format
//! dispatch, consulted after the probe names a container.
//!
//! Wiring ONE container family = creating `containers/<fmt>.rs` (a
//! `pub(crate) fn open(stream, opts) -> Result<OpenedMedia,
//! CommandError>` that constructs the demuxer, runs the pre-pass,
//! harvests the container's track facts and wraps the demuxer in the
//! shared [`SelectDemuxer`](crate::demux_host::SelectDemuxer)) plus
//! exactly **one `mod <fmt>;` declaration and one match arm** here —
//! no other shared file is touched by a wiring unit (the collision
//! rule; merge conflicts are confined to this file's two append
//! points and are trivial one-line re-merges). Entries keep the §5
//! unit order: wav → aiff → adts → ogg → ivf → mp4 → mkv → ts → ps →
//! es (`avi` is wired here as the moved first vertical).
//!
//! Anything not wired falls to [`unwired_refusal`] — the honest typed
//! refusal (stage `probe`, the format named, exit 65), kept forever:
//! a probing-but-unwired container is an honest no, never a silent
//! fallback.

mod adts;
mod aiff;
mod avi;
mod es;
mod ivf;
mod mkv;
mod mp4;
mod ogg;
mod ts;
mod wav;

use commands::{CommandError, ErrorCode, Stage};
use plugin_api::SourceStream;

use crate::demux_host::{OpenOptions, OpenedMedia};

/// THE dispatch: the probed format id → its wired container host.
///
/// # Errors
/// [`CommandError`] (stage `probe`, code `unsupported`, the format
/// named) for every family without a wiring unit yet — the registry's
/// honest default arm.
pub(crate) fn open(
    format: &str,
    stream: Box<dyn SourceStream>,
    opts: &OpenOptions,
) -> Result<OpenedMedia, CommandError> {
    match format {
        // The entry shape: `"<probe-id>" => <mod>::open(stream, opts),`
        "adts" => adts::open(stream, opts),
        "aiff" => aiff::open(stream, opts),
        "avi" => avi::open(stream, opts),
        "ivf" => ivf::open(stream, opts),
        // The `matroska` id covers mkv/mka/webm (6.OPW-mkv).
        "matroska" => mkv::open(stream, opts),
        // The raw-ES family (6.OPW-es): one module, five probe ids
        // (the probe's layer/NAL-set discrimination named the lane;
        // the `annexb` family id never reaches the registry — the
        // probe mints it sub-threshold for ambiguous NAL shapes).
        "flac" => es::open_flac(stream, opts),
        "hevc" => es::open_annexb(demux::es::annexb::AnnexbCodec::Hevc, stream, opts),
        "h264" => es::open_annexb(demux::es::annexb::AnnexbCodec::H264, stream, opts),
        "mp2" => es::open_mpeg_audio(stream, opts, format),
        "mp3" => es::open_mpeg_audio(stream, opts, format),
        "mp4" => mp4::open(stream, opts),
        "ogg" => ogg::open(stream, opts),
        "mpegts" => ts::open(stream, opts),
        "wav" => wav::open(stream, opts),
        other => Err(unwired_refusal(other)),
    }
}

/// The registry's default arm: today's gate message shape, kept (the
/// format is named, the wiring state is named — the D-018-era
/// refusal, `demux_host.rs`'s former AVI-only gate).
fn unwired_refusal(format: &str) -> CommandError {
    CommandError::new(
        Stage::Probe,
        ErrorCode::Unsupported,
        format!(
            "format `{format}` probes but has no open-path wiring yet \
             (R3 registered the signature; the container-host registry wires \
             one family per unit — this one has not landed)"
        ),
    )
    .with_detail("format", format)
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_types::ids::SourceUri;
    use testkit::Must;

    /// The default arm's honest refusal (§4.6): an unwired format
    /// through the registry is the typed probe-stage refusal naming
    /// the format — the same message shape the D-018 gate answered
    /// with. The stream is never read: the refusal precedes any
    /// container construction. (6.OPW-ts wired the ts family, so
    /// 6.OPW-es re-targets this pin from `mpegts` to the
    /// still-unwired `mpegps` — a REAL probe id, the ps/vob
    /// family's, the LAST family in the §5 wiring order; the
    /// 6.OPW-ps unit re-pins it when it lands.)
    #[test]
    fn unwired_format_is_refused_with_the_format_named() {
        let uri = SourceUri::parse("file:///tmp/any.mpg").must();
        let opts = OpenOptions {
            uri,
            start_time_us: None,
            no_video: false,
            no_audio: false,
        };
        let stream = demux::avi::test_support::mem_stream(*b"never read").must();
        match open("mpegps", stream, &opts) {
            Err(err) => {
                assert_eq!(err.stage, Stage::Probe, "{err}");
                assert_eq!(err.code, ErrorCode::Unsupported, "{err}");
                assert!(err.message.contains("format `mpegps` probes"), "{err}");
                assert!(err.message.contains("no open-path wiring"), "{err}");
                assert_eq!(
                    err.details.get("format").map(String::as_str),
                    Some("mpegps"),
                    "{err}"
                );
            }
            Ok(_) => unreachable!("mpegps has no wiring unit yet"),
        }
    }
}
