//! The signature engine (doc 03 §4.2): the [`ProbePlugin`] seam every
//! format signature implements, and the [`ProbeRegistry`] that scores
//! them and applies the selection rules.
//!
//! # Selection rules (doc 03 §4.2(e))
//!
//! Scoring is a **pure function of `(prefix, meta)`** — plugins hold no
//! stream state and are handed only the window and the advisory meta.
//! The winner is the hit with the highest score; ties break by plugin
//! priority (the manifest figure, doc 06 §2.2), then by the **shorter
//! prefix consumed** `[PROPOSAL]`. A hit whose score exceeds 100
//! violates the documented 0..=100 range and is **discarded, never
//! clamped** (a wrong number stays wrong); the doc 06 §7 conformance
//! kit will flag such plugins loudly once extension loading exists.
//!
//! # Threshold (doc 03 §4.2(d))
//!
//! A best hit scoring below `min_score` is a typed `Unsupported`
//! carrying whatever is known — the best hit's format name, the
//! sniffed bytes, the source hints. Never a guess presented as fact.

mod avi;
mod mp4;

use media_types::EngineError;
use plugin_api::manifest::DEFAULT_PRIORITY_BUILTIN;

pub use avi::AviSignature;
pub use mp4::Mp4Signature;

use crate::config::ProbeConfig;
use crate::error;
use crate::report::{Confidence, ProbeHit, ProbeReport, SourceMeta};

/// One format signature (doc 03 §4.2 `ProbePlugin`).
///
/// Implementations are anchored, bounded scans over the probe window:
/// they check their magic at the window start and walk only as deep as
/// their format's header structures, clamped to the window. They never
/// panic (panics are contract violations, the plugin-api rule), never
/// allocate unboundedly (hints are capped by the walk guards below),
/// and never touch a stream — the window is all they see.
pub trait ProbePlugin: Send {
    /// Score `prefix` (the probe window) against this signature.
    ///
    /// Returns [`None`] when the signature does not anchor at all; a
    /// hit otherwise (possibly a sub-threshold one — the registry
    /// applies the minimum, doc 03 §4.2(d), so the no-hit error can
    /// still name the family).
    fn probe(&self, prefix: &[u8], meta: &SourceMeta) -> Option<ProbeHit>;
}

/// Iteration guard for one chunk/box walk (a corrupted size table
/// cannot spin the scanner; sizes that stall progress end the walk).
/// `[PROPOSAL]` — bounded work is the contract; the figure is a
/// safety net far above any real header's child count.
pub(crate) const MAX_WALK_STEPS: usize = 256;

/// The scoring ladder the builtin signatures award (doc 03 §4.2
/// `score: u8 /* 0..=100 */`).
///
/// `[PROPOSAL]` — the ladder's rungs, their distances and the gap to
/// the doc's `min_score = 60` default are this card's invention,
/// pinned by tests:
///
/// | rung | meaning |
/// |------|---------|
/// | [`SCORE_MAGIC_ANCHOR`] | magic anchored — the format family is certain enough to select a demuxer |
/// | [`SCORE_COMPAT_BRAND`] | MP4: only a compatible brand names the family |
/// | [`SCORE_CONTAINER_STRUCTURE`] | the primary container structure is confirmed (`LIST hdrl` / `moov`) |
/// | [`SCORE_PRIMARY_HEADER`] | the primary header chunk is confirmed (`avih` / `mvhd`) |
/// | [`SCORE_TRACK_HINTS`] | track content was sniffed (`strl` pairs / `stsd` entries) |
///
/// 100 stays unawarded in v0: reserved for a signature with no
/// unknowns left (a checksummed header); the builtin ceiling is 95.
pub(crate) const SCORE_MAGIC_ANCHOR: u8 = 70;
pub(crate) const SCORE_COMPAT_BRAND: u8 = 65;
pub(crate) const SCORE_FAMILY_ONLY: u8 = 50;
pub(crate) const SCORE_CONTAINER_STRUCTURE: u8 = 80;
pub(crate) const SCORE_PRIMARY_HEADER: u8 = 90;
pub(crate) const SCORE_TRACK_HINTS: u8 = 95;

/// One registered signature: the plugin and its manifest priority.
struct Entry {
    plugin: Box<dyn ProbePlugin>,
    priority: u32,
}

/// The scored signature set (the MP §6 registry-selection seam; doc 06
/// §3's full registry is the Phase-3 engine card — this is the probe's
/// own ranked list).
#[derive(Default)]
pub struct ProbeRegistry {
    entries: Vec<Entry>,
}

impl ProbeRegistry {
    /// An empty registry (extension and test path).
    pub fn empty() -> ProbeRegistry {
        ProbeRegistry {
            entries: Vec::new(),
        }
    }

    /// The builtin signature set at the builtin priority (doc 06 §2.2
    /// `DEFAULT_PRIORITY_BUILTIN`): AVI and MP4 — the fixture corpus
    /// formats of the first vertical (D-018 variant A) and its
    /// decision-independent sibling (2.A3).
    pub fn with_builtins() -> ProbeRegistry {
        let mut registry = ProbeRegistry::empty();
        registry.register(Box::new(AviSignature), DEFAULT_PRIORITY_BUILTIN);
        registry.register(Box::new(Mp4Signature), DEFAULT_PRIORITY_BUILTIN);
        registry
    }

    /// Add a signature under `priority` (higher wins ties).
    pub fn register(&mut self, plugin: Box<dyn ProbePlugin>, priority: u32) {
        self.entries.push(Entry { plugin, priority });
    }

    /// How many signatures are registered.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no signature is registered.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Score a probe window: the pure selection pass (doc 03 §4.2(e)).
    ///
    /// Runs every signature over `(prefix, meta)`, picks the winner by
    /// score → priority → shorter prefix consumed, and applies the
    /// configured minimum.
    ///
    /// # Errors
    /// [`EngineError`] (stage [`PipelineStage::Probe`], kind
    /// `Unsupported`) when no hit meets `min_score` — carrying the
    /// best-known format name, the sniffed bytes and the source hints
    /// (doc 03 §4.2(d)).
    pub fn probe(
        &self,
        prefix: &[u8],
        meta: &SourceMeta,
        config: &ProbeConfig,
    ) -> Result<ProbeReport, EngineError> {
        // (hit, priority, validated confidence) — the score range is
        // enforced at collection, so the winner's confidence is always
        // representable below.
        let mut best: Option<(ProbeHit, u32, Confidence)> = None;
        for entry in &self.entries {
            if let Some(hit) = entry.plugin.probe(prefix, meta)
                && let Some(confidence) = Confidence::from_score(hit.score)
            {
                let replace = match &best {
                    None => true,
                    Some((current, current_priority, _)) => {
                        better_tiebreak(&hit, entry.priority, current, *current_priority)
                    }
                };
                if replace {
                    best = Some((hit, entry.priority, confidence));
                }
            }
        }
        match best {
            Some((hit, _, confidence)) if confidence.meets(config.min_score) => {
                Ok(ProbeReport {
                    format: hit.format,
                    confidence,
                    track_hints: hit.track_hints,
                    bytes_examined: prefix.len(),
                })
            }
            other => Err(error::no_demuxer(
                other.as_ref().map(|(hit, _, _)| hit),
                prefix,
                meta,
            )),
        }
    }
}

/// Doc 03 §4.2(e) tie-break: score, then priority, then shorter prefix.
fn better_tiebreak(new: &ProbeHit, new_priority: u32, old: &ProbeHit, old_priority: u32) -> bool {
    new.score > old.score
        || (new.score == old.score
            && (new_priority > old_priority
                || (new_priority == old_priority && new.prefix_consumed < old.prefix_consumed)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::{FormatId, TrackHint};
    use media_types::PipelineStage;
    use media_types::ids::MediaKind;

    fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
        match r {
            Ok(v) => v,
            Err(e) => unreachable!("{e}"),
        }
    }

    /// A test signature with a fixed verdict (the tie-break machinery
    /// needs plugins whose behavior is the test's own data).
    struct Fixed {
        format: &'static str,
        score: u8,
        prefix_consumed: usize,
        anchors: bool,
    }

    impl ProbePlugin for Fixed {
        fn probe(&self, prefix: &[u8], _meta: &SourceMeta) -> Option<ProbeHit> {
            if !self.anchors || prefix.is_empty() {
                return None;
            }
            Some(ProbeHit {
                format: ok(FormatId::parse(self.format)),
                score: self.score,
                track_hints: Vec::new(),
                prefix_consumed: self.prefix_consumed,
            })
        }
    }

    fn registry(entries: Vec<(Fixed, u32)>) -> ProbeRegistry {
        let mut r = ProbeRegistry::empty();
        for (fixed, priority) in entries {
            r.register(Box::new(fixed), priority);
        }
        r
    }

    fn config(min_score: u8) -> ProbeConfig {
        ok(ProbeConfig::new(4096, min_score))
    }

    #[test]
    fn higher_score_wins_over_priority() {
        let r = registry(vec![
            (Fixed { format: "aaa", score: 70, prefix_consumed: 100, anchors: true }, 999),
            (Fixed { format: "bbb", score: 90, prefix_consumed: 100, anchors: true }, 1),
        ]);
        let report = ok(r.probe(b"x", &SourceMeta::new(None, None), &config(60)));
        assert_eq!(report.format.as_str(), "bbb");
    }

    #[test]
    fn priority_breaks_score_ties() {
        let r = registry(vec![
            (Fixed { format: "aaa", score: 90, prefix_consumed: 1, anchors: true }, 10),
            (Fixed { format: "bbb", score: 90, prefix_consumed: 1, anchors: true }, 20),
        ]);
        let report = ok(r.probe(b"x", &SourceMeta::new(None, None), &config(60)));
        assert_eq!(report.format.as_str(), "bbb", "higher priority wins the tie");
    }

    #[test]
    fn shorter_prefix_breaks_priority_ties() {
        let r = registry(vec![
            (Fixed { format: "aaa", score: 90, prefix_consumed: 50, anchors: true }, 30),
            (Fixed { format: "bbb", score: 90, prefix_consumed: 10, anchors: true }, 30),
        ]);
        let report = ok(r.probe(b"x", &SourceMeta::new(None, None), &config(60)));
        assert_eq!(report.format.as_str(), "bbb", "smaller prefix_consumed wins");
    }

    #[test]
    fn subthreshold_best_hit_names_the_family_in_the_error() {
        let r = registry(vec![(Fixed { format: "mp4", score: 50, prefix_consumed: 8, anchors: true }, 100)]);
        let e = match r.probe(b"somebytes", &SourceMeta::new(Some(9), Some("bin")), &config(60)) {
            Ok(_) => unreachable!("50 < 60 must reject"),
            Err(e) => e,
        };
        assert_eq!(e.context.code, "probe.no-demuxer");
        assert_eq!(e.stage, PipelineStage::Probe);
        match e.kind {
            media_types::ErrorKind::Unsupported { ref format } => {
                let info = match format {
                    Some(i) => i,
                    None => unreachable!("info present"),
                };
                assert_eq!(info.name.as_deref(), Some("mp4"));
                assert_eq!(info.sniffed, b"somebytes".to_vec());
            }
            other => unreachable!("wrong kind: {other:?}"),
        }
        assert!(std::error::Error::source(&e).is_some(), "hints ride the chain");
    }

    #[test]
    fn out_of_range_scores_are_discarded_not_clamped() {
        // 101 violates the 0..=100 contract; the hit must neither win
        // nor reach the error payload (a clamped 100 would be a lie).
        let r = registry(vec![
            (Fixed { format: "liar", score: 101, prefix_consumed: 1, anchors: true }, 100),
            (Fixed { format: "honest", score: 70, prefix_consumed: 1, anchors: true }, 1),
        ]);
        let report = ok(r.probe(b"x", &SourceMeta::new(None, None), &config(60)));
        assert_eq!(report.format.as_str(), "honest");
        // And with no honest competitor: a plain unknown-format error.
        let r = registry(vec![(Fixed { format: "liar", score: 101, prefix_consumed: 1, anchors: true }, 100)]);
        let e = match r.probe(b"x", &SourceMeta::new(None, None), &config(60)) {
            Ok(_) => unreachable!("must reject"),
            Err(e) => e,
        };
        match e.kind {
            media_types::ErrorKind::Unsupported { ref format } => {
                let info = match format {
                    Some(i) => i,
                    None => unreachable!("info present"),
                };
                assert!(info.name.is_none(), "the liar is not named");
            }
            other => unreachable!("wrong kind: {other:?}"),
        }
    }

    #[test]
    fn empty_registry_is_a_clean_unknown() {
        let r = ProbeRegistry::empty();
        assert!(r.is_empty());
        let e = match r.probe(b"RIFF....AVI ", &SourceMeta::new(None, None), &config(60)) {
            Ok(_) => unreachable!("must reject"),
            Err(e) => e,
        };
        assert_eq!(e.context.code, "probe.no-demuxer");
    }

    #[test]
    fn builtin_registry_shape() {
        let r = ProbeRegistry::with_builtins();
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn builtin_signatures_never_panic_on_arbitrary_windows() {
        // Doc 03 §4.2(f): random prefixes must never panic any probe
        // plugin. Deterministic sweep: every byte value at every
        // position of two template windows, plus every truncation.
        let r = ProbeRegistry::with_builtins();
        let meta = SourceMeta::new(None, None);
        let cfg = config(60);
        let mut template = vec![0u8; 64];
        template[..4].copy_from_slice(b"RIFF");
        template[8..12].copy_from_slice(b"AVI ");
        template[16..20].copy_from_slice(b"moov");
        for pos in 0..template.len() {
            for value in 0..=255u8 {
                let mut window = template.clone();
                window[pos] = value;
                let _ = r.probe(&window, &meta, &cfg);
            }
        }
        for cut in 0..=template.len() {
            let _ = r.probe(&template[..cut], &meta, &cfg);
        }
    }

    #[test]
    fn track_hints_flow_from_hit_to_report() {
        struct Hinting;
        impl ProbePlugin for Hinting {
            fn probe(&self, prefix: &[u8], _meta: &SourceMeta) -> Option<ProbeHit> {
                if prefix.is_empty() {
                    return None;
                }
                Some(ProbeHit {
                    format: FormatId::avi(),
                    score: 95,
                    track_hints: vec![ok(TrackHint::new(MediaKind::Video, "mjpeg"))],
                    prefix_consumed: 4,
                })
            }
        }
        let mut r = ProbeRegistry::empty();
        r.register(Box::new(Hinting), DEFAULT_PRIORITY_BUILTIN);
        let report = ok(r.probe(b"x", &SourceMeta::new(None, None), &config(60)));
        assert_eq!(report.track_hints.len(), 1);
        assert_eq!(report.track_hints[0].codec.as_str(), "mjpeg");
        assert_eq!(report.bytes_examined, 1);
    }
}
