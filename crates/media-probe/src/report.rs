//! The probe outcome vocabulary (doc 03 §4.2; card 2.A2):
//! [`FormatId`], [`Confidence`], [`TrackHint`], [`ProbeHit`],
//! [`ProbeReport`] and [`SourceMeta`] — the data every signature speaks
//! and the engine consumes when selecting a demuxer.
//!
//! House rules (MP §8): a value and its absence are different types
//! (`SourceMeta::size` is an [`Option`], never a `0` sentinel); every
//! public constructor validates and returns
//! [`Result`](std::result::Result); nothing here allocates
//! unboundedly (the only owned payloads are short id strings and the
//! bounded hint vector a signature produced while scanning a
//! window-bounded prefix).

use media_types::EngineError;
use media_types::ids::{CodecDescriptor, MediaKind, SourceUri};
use plugin_api::SourceStream;

use crate::error;

/// The maximum confidence score (doc 03 §4.2: `score: u8 /* 0..=100 */`).
pub const CONFIDENCE_MAX: u8 = 100;

/// A container format identity (doc 03 §4.2 `FormatId`).
///
/// An **open** validated newtype, the [`Scheme`](media_types::ids::Scheme)
/// house pattern: formats are plugin data (each demuxer manifest names
/// the formats it serves), so the vocabulary is a string, not a closed
/// enum — but every id is grammar-checked at construction. The builtin
/// signatures mint their ids through [`FormatId::avi`] /
/// [`FormatId::mp4`]; extension manifests go through
/// [`FormatId::parse`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FormatId(String);

impl FormatId {
    /// The AVI container (RIFF, form `AVI `; doc 03 §7 first vertical).
    pub fn avi() -> FormatId {
        FormatId("avi".to_owned())
    }

    /// The MP4 / ISO-BMFF container family (`ftyp` box; doc 03 §7's
    /// decision-independent second variant).
    pub fn mp4() -> FormatId {
        FormatId("mp4".to_owned())
    }

    /// The WAV container (RIFF/RF64/BW64, form `WAVE`; card 5.A2).
    pub fn wav() -> FormatId {
        FormatId("wav".to_owned())
    }

    /// The AIFF/AIFC container (`FORM` magic; card 5.A2).
    pub fn aiff() -> FormatId {
        FormatId("aiff".to_owned())
    }

    /// The Ogg container family (`OggS` capture; card 5.B1).
    pub fn ogg() -> FormatId {
        FormatId("ogg".to_owned())
    }

    /// The Matroska/WebM container family (EBML header; card 5.D1).
    pub fn matroska() -> FormatId {
        FormatId("matroska".to_owned())
    }

    /// The ADTS/ADIF raw-AAC container (card 5.C2b).
    pub fn adts() -> FormatId {
        FormatId("adts".to_owned())
    }

    /// The MPEG transport stream (card 5.I1).
    pub fn mpegts() -> FormatId {
        FormatId("mpegts".to_owned())
    }

    /// The MPEG program stream / DVD-VOB (card 5.I2).
    pub fn mpegps() -> FormatId {
        FormatId("mpegps".to_owned())
    }

    /// The IVF delivery container (`DKIF`; card 5.I3b).
    pub fn ivf() -> FormatId {
        FormatId("ivf".to_owned())
    }

    /// The raw MPEG-audio Layer III elementary stream (`mp3`; card
    /// 6.OPW-es — the layer-01 half of the MPEG-audio syncword
    /// discrimination, disjoint from ADTS's layer-00).
    pub fn mp3() -> FormatId {
        FormatId("mp3".to_owned())
    }

    /// The raw MPEG-audio Layer II elementary stream (`mp2`; card
    /// 6.OPW-es — the layer-10 half of the same discrimination).
    pub fn mp2() -> FormatId {
        FormatId("mp2".to_owned())
    }

    /// The Annex-B raw video family (`.264`/`.265`; card 6.OPW-es —
    /// a **sub-threshold family id**: minted only when the window's
    /// NAL shapes fit both codec hypotheses and never selected by
    /// the registry's default minimum; it names the family in the
    /// refusal, never a codec guess).
    pub fn annexb() -> FormatId {
        FormatId("annexb".to_owned())
    }

    /// The raw H.264 Annex-B elementary stream (card 6.OPW-es —
    /// decisive when the window's NAL headers are HEVC-impossible).
    pub fn h264() -> FormatId {
        FormatId("h264".to_owned())
    }

    /// The raw HEVC Annex-B elementary stream (card 6.OPW-es —
    /// decisive when the window's NAL headers are H.264-impossible).
    pub fn hevc() -> FormatId {
        FormatId("hevc".to_owned())
    }

    /// The native FLAC stream (`fLaC` marker; card 6.OPW-es).
    pub fn flac() -> FormatId {
        FormatId("flac".to_owned())
    }

    /// Validate a format id string (extension path).
    ///
    /// Grammar: 1..=24 bytes of ASCII lowercase letters, digits, `-`
    /// and `.` — the plugin-manifest vocabulary style.
    ///
    /// # Errors
    /// [`EngineError`] (stage [`PipelineStage::Probe`], kind
    /// `Unsupported`) when the string is empty, too long, or contains
    /// characters outside the grammar.
    pub fn parse(s: &str) -> Result<FormatId, EngineError> {
        let valid = !s.is_empty()
            && s.len() <= FORMAT_ID_MAX_BYTES
            && s.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.');
        if !valid {
            return Err(error::format_id_invalid());
        }
        Ok(FormatId(s.to_owned()))
    }

    /// The id string (already user-renderable; never a secret).
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Maximum length of a [`FormatId`] string, in bytes.
const FORMAT_ID_MAX_BYTES: usize = 24;

impl std::fmt::Display for FormatId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A signature's confidence in its verdict, 0..=[`CONFIDENCE_MAX`]
/// (doc 03 §4.2's `score` with the range made unrepresentable-wrong).
///
/// Built only through [`Confidence::from_score`], which rejects
/// out-of-range values instead of clamping them (a plugin reporting a
/// score above 100 violates the documented contract; the registry
/// discards that hit rather than trust a mis-scaled number).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Confidence(u8);

impl Confidence {
    /// Wrap a raw score, rejecting anything above
    /// [`CONFIDENCE_MAX`](CONFIDENCE_MAX).
    ///
    /// Returns [`None`] for out-of-range scores — a contract violation
    /// by the scoring plugin, surfaced to the registry as a dropped
    /// hit (never clamped: a wrong number stays wrong).
    pub fn from_score(score: u8) -> Option<Confidence> {
        if score <= CONFIDENCE_MAX {
            Some(Confidence(score))
        } else {
            None
        }
    }

    /// The numeric score, 0..=[`CONFIDENCE_MAX`](CONFIDENCE_MAX).
    pub fn as_u8(self) -> u8 {
        self.0
    }

    /// Whether this confidence meets a configured minimum score.
    pub fn meets(self, min_score: u8) -> bool {
        self.0 >= min_score
    }
}

impl std::fmt::Display for Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.0, CONFIDENCE_MAX)
    }
}

/// One sniffed track: what the signature saw in the probe window,
/// expressed in the L0 vocabulary the demuxer will confirm.
///
/// A *hint*, not a track: the probe is the cheap pre-stage (doc 03
/// §4.2(a) — "identify the container", not "parse it"); the selected
/// demuxer's `start` owns track truth. Hints carry exactly what the
/// signature's anchored scan could see — an AVI `strl` pair or an MP4
/// `hdlr`+`stsd` pair — and are absent when the window ends before
/// them or damage makes them unreadable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackHint {
    /// What the track carries (from the container's own stream-type
    /// marker: AVI `strh` fccType `vids`/`auds`, MP4 `hdlr`
    /// handler_type `vide`/`soun`).
    pub kind: MediaKind,
    /// The codec the container declares for it, as the registry
    /// descriptor string (`mjpeg`, `pcm_s16le` — the same names the
    /// demuxers issue; the expectation TOMLs record them).
    pub codec: CodecDescriptor,
}

impl TrackHint {
    /// Build a hint from a kind and a codec descriptor string.
    ///
    /// # Errors
    /// [`EngineError`] (stage [`PipelineStage::Probe`]) when the
    /// descriptor string is not a valid [`CodecDescriptor`] — the
    /// L0 grammar applies unchanged at this boundary.
    pub fn new(kind: MediaKind, codec: &str) -> Result<TrackHint, EngineError> {
        match CodecDescriptor::new(codec) {
            Ok(codec) => Ok(TrackHint { kind, codec }),
            Err(_) => Err(error::hint_codec_invalid()),
        }
    }
}

/// One signature's full claim over a probe window (doc 03 §4.2
/// `ProbeHit`, extended with the card's track-hint payload: the hints
/// are a by-product of the same scan that produces the score, and the
/// report needs them).
///
/// Field order is tie-break order: score first (doc 03 §4.2(e) —
/// higher wins), then the registry's plugin priority, then the shorter
/// prefix consumed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeHit {
    /// The format this signature identified.
    pub format: FormatId,
    /// Confidence 0..=100 (validated when the registry accepts the
    /// hit; see [`Confidence::from_score`]).
    pub score: u8,
    /// Track hints sniffed during the scan (possibly empty — a
    /// magic-only hit has none).
    pub track_hints: Vec<TrackHint>,
    /// How many leading window bytes the signature actually inspected
    /// to reach its verdict (doc 03 §4.2(e) tie-break: shorter wins).
    pub prefix_consumed: usize,
}

/// The outcome of a successful probe (doc 01 §2 step 3: "a ProbeReport
/// (format, tracks, metadata)"; card 2.A2's shape: format id,
/// confidence, track hints).
///
/// Metadata extraction (tags, cover art — the kind-14 plugin surface
/// whose builtin home is this crate) is a later card; v0 reports the
/// format, the confidence and the track hints only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeReport {
    /// The identified container format.
    pub format: FormatId,
    /// The winning signature's confidence, 0..=100.
    pub confidence: Confidence,
    /// The sniffed track hints (advisory; the demuxer owns truth).
    pub track_hints: Vec<TrackHint>,
    /// How many bytes the probe examined — the window actually read,
    /// always ≤ the configured `max_lookahead` (the depth bound made
    /// observable).
    pub bytes_examined: usize,
}

/// What the probe knows about a source before scoring: size and an
/// extension hint (doc 03 §4.2's `SourceMeta`).
///
/// The extension is a **hint, not truth** (doc 03 §4.2(f)): content
/// wins, always — a valid AVI named `.mp4` probes as AVI, and the
/// extension rides only the no-hit error payload (a mime hint for the
/// user), never the scoring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMeta {
    /// Total source size when known — live sources report `None`
    /// (a value and its absence are different types, MP §8).
    pub size: Option<u64>,
    /// Lowercased, dot-stripped file extension when the URI carries a
    /// plausible one; `None` otherwise. Advisory only.
    pub extension: Option<String>,
}

impl SourceMeta {
    /// Build meta from parts (the test / manual path).
    pub fn new(size: Option<u64>, extension: Option<&str>) -> SourceMeta {
        SourceMeta {
            size,
            extension: extension.map(|e| e.to_ascii_lowercase()),
        }
    }

    /// Collect meta from an opened stream: live `size` plus the
    /// extension parsed out of the stream's URI.
    ///
    /// The extension parse is deliberately naive (last path segment,
    /// text after the final dot, 1..=16 ASCII alphanumerics, no
    /// percent-decoding): it is a display hint with no decision power,
    /// so URI edge cases cost nothing.
    pub fn from_stream(stream: &dyn SourceStream) -> SourceMeta {
        SourceMeta {
            size: stream.size(),
            extension: extension_of(stream.uri()),
        }
    }
}

/// Extract a plausible extension from a URI's raw text, if any.
fn extension_of(uri: &SourceUri) -> Option<String> {
    let raw = uri.raw();
    let after_scheme = raw.split_once("://")?;
    let path = after_scheme.1.split(['?', '#']).next()?;
    let file = path.rsplit('/').next()?;
    // A leading dot marks a hidden file, not an extension (`/.hidden`
    // has none); only a later dot introduces one (`.config.toml` → toml).
    let (_, ext) = file.rsplit_once('.')?;
    if file.starts_with('.') && !file[1..].contains('.') {
        return None;
    }
    if ext.is_empty() || ext.len() > 16 || !ext.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    Some(ext.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_types::PipelineStage;

    /// The unit-test unwrapping convention (testkit's `Must` vocabulary
    /// inlined — src-side unit tests stay dependency-light, the demux
    /// precedent): a constructor failing on a literal is a bug, not a
    /// runtime condition.
    fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
        match r {
            Ok(v) => v,
            Err(e) => unreachable!("literal must parse: {e}"),
        }
    }

    fn err<T, E>(r: Result<T, E>) -> E {
        match r {
            Ok(_) => unreachable!("must fail"),
            Err(e) => e,
        }
    }

    #[test]
    fn format_id_valid_shapes_parse() {
        for s in ["avi", "mp4", "mpeg-ts", "a.b", "x2"] {
            let id = ok(FormatId::parse(s));
            assert_eq!(id.as_str(), s);
        }
    }

    #[test]
    fn format_id_rejects_bad_shapes() {
        for s in [
            "",
            "AVI",
            "mp4 container",
            "way-too-long-format-id-string",
            "a\tb",
        ] {
            let e = err(FormatId::parse(s));
            assert_eq!(e.stage, PipelineStage::Probe);
            assert_eq!(e.context.code, "probe.format-id.invalid");
        }
    }

    #[test]
    fn builtin_format_ids_are_the_documented_names() {
        assert_eq!(FormatId::avi().as_str(), "avi");
        assert_eq!(FormatId::mp4().as_str(), "mp4");
        // The constructors satisfy the public grammar (they are the
        // same vocabulary an extension manifest would declare).
        assert_eq!(ok(FormatId::parse("avi")), FormatId::avi());
        assert_eq!(ok(FormatId::parse("mp4")), FormatId::mp4());
    }

    #[test]
    fn confidence_bounds_are_enforced_not_clamped() {
        assert_eq!(Confidence::from_score(0).map(Confidence::as_u8), Some(0));
        assert_eq!(
            Confidence::from_score(CONFIDENCE_MAX).map(Confidence::as_u8),
            Some(100)
        );
        assert!(Confidence::from_score(CONFIDENCE_MAX + 1).is_none());
        assert!(Confidence::from_score(255).is_none());
        // meet-or-exceed semantics against a minimum score.
        let c = Confidence::from_score(60).must_conf();
        assert!(c.meets(60));
        assert!(!c.meets(61));
    }

    #[test]
    fn track_hint_validates_its_descriptor() {
        let h = ok(TrackHint::new(MediaKind::Video, "mjpeg"));
        assert_eq!(h.kind, MediaKind::Video);
        assert_eq!(h.codec.as_str(), "mjpeg");
        let e = err(TrackHint::new(MediaKind::Audio, ""));
        assert_eq!(e.context.code, "probe.hint-codec-invalid");
    }

    #[test]
    fn source_meta_extension_extraction() {
        // Uppercase is lowered; query strings are not part of the name.
        let uri = ok(SourceUri::parse("file:///media/Movies/Clip.AVI?x=1"));
        let meta = SourceMeta::from_stream(&crate::contract::CountingSource::with_uri(
            b"RIFF....AVI ",
            &uri,
        ));
        assert_eq!(meta.extension.as_deref(), Some("avi"));
        assert_eq!(meta.size, Some(12));
        // No extension, dotfile, double suffix, trailing slash.
        for (raw, want) in [
            ("file:///a/b/movie", None),
            ("file:///a/b/.hidden", None),
            ("file:///a/b/archive.tar.gz", Some("gz")),
            ("file:///a/b/x.mp4/", None),
        ] {
            let uri = ok(SourceUri::parse(raw));
            let meta = SourceMeta::from_stream(&crate::contract::CountingSource::with_uri(
                b"1234567890",
                &uri,
            ));
            assert_eq!(meta.extension.as_deref(), want, "{raw}");
        }
    }

    #[test]
    fn probe_report_carries_the_card_shape() {
        let report = ProbeReport {
            format: FormatId::avi(),
            confidence: Confidence::from_score(95).must_conf(),
            track_hints: vec![ok(TrackHint::new(MediaKind::Video, "mjpeg"))],
            bytes_examined: 312,
        };
        assert_eq!(report.format.as_str(), "avi");
        assert_eq!(report.confidence.as_u8(), 95);
        assert_eq!(report.track_hints.len(), 1);
        assert_eq!(report.bytes_examined, 312);
        assert_eq!(report.confidence.to_string(), "95/100");
    }

    /// Test-only confidence unwrap (in-range literals only).
    trait MustConf {
        fn must_conf(self) -> Confidence;
    }
    impl MustConf for Option<Confidence> {
        fn must_conf(self) -> Confidence {
            match self {
                Some(c) => c,
                None => unreachable!("literal score is in range"),
            }
        }
    }
}
