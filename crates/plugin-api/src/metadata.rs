//! The metadata-provider contract (doc 06 §1.2 kind 14): tag / cover
//! art extraction at probe time, plus on-demand lookups. Builtin
//! home: `media-probe` (L2); allowed as a WASM extension in v1 (§1.4).
//!
//! Instance scope: app singleton (§1.3). Extraction is a **pull from
//! bytes the engine already holds** (the probe window); lookups are
//! **on-demand requests** the engine issues (never a background
//! network loop the plugin owns). MP §12: outbound network happens
//! only through the host fetcher under the `http-fetch` permission
//! and its host allowlist — a plugin never opens sockets.

use std::collections::BTreeMap;

use media_types::packet::SharedBytes;
use media_types::{EngineError, PipelineStage};

/// One metadata value: a tag, a picture blob, a lyric text.
///
/// Keys are the doc 06 §2.2 capability vocabulary (`tags`,
/// `cover-art`, `lyrics`); values are plain UTF-8 or opaque bytes —
/// a value and its absence are different types (`Option`), never a
/// sentinel (MP §8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Metadata {
    /// A textual tag (`title`, `artist`, `album`, ...).
    Tag(Tag),
    /// Cover art with its declared MIME type.
    CoverArt {
        /// MIME type as declared by the source document.
        mime: String,
        /// The image bytes (shared, zero-copy).
        data: SharedBytes,
    },
    /// Synchronized or plain lyrics text.
    Lyrics(String),
}

/// A named textual tag (doc 06 §2.2 `provides = ["tags", ...]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    /// Tag name, lowercase with `-` separators (the manifest
    /// vocabulary style; e.g. `title`, `album-artist`).
    pub name: String,
    /// The tag value (never empty — an empty value is no value).
    pub value: String,
}

/// A metadata provider (doc 06 §1.2 row 14).
///
/// # Calling context
///
/// Worker thread ([`Send`]); bounded work per call — one probe window
/// or one lookup per call, never an internal fan-out over many
/// sources; network lookups ride the engine's timeout budget and
/// surface `NetworkCause` errors. Panics are contract violations.
pub trait MetadataProvider: Send {
    /// Extract what this provider can from a probe window (the first
    /// bytes of the media, already read by the engine — the provider
    /// does no I/O here).
    ///
    /// # Errors
    /// [`EngineError`] (stage [`PipelineStage::Probe`]) when the
    /// bytes fail structurally; absence of metadata is `Ok(vec![])`,
    /// not an error (MP §8: absence is data).
    fn extract(&mut self, probe_window: &SharedBytes) -> Result<Vec<Metadata>, EngineError>;

    /// The lookup kinds this provider serves on demand (must be a
    /// subset of the manifest's `provides` claim — doc 06 §7.2
    /// capability honesty).
    fn provides(&self) -> Vec<&'static str>;

    /// Run one on-demand lookup (e.g. MusicBrainz tags for a TOC).
    /// The engine calls this only in response to a user action or an
    /// explicit settings-driven prefetch — never speculatively.
    ///
    /// # Errors
    /// [`EngineError`] (stage [`PipelineStage::Plugin`]) — network
    /// causes for fetch failures; `Unsupported` when the query kind
    /// is not served.
    fn lookup(
        &mut self,
        kind: &str,
        query: &BTreeMap<String, String>,
    ) -> Result<Vec<Metadata>, EngineError>;
}

/// Typed provider error: lookup kind outside the served set.
pub fn lookup_not_served(kind: &str) -> EngineError {
    EngineError::new(
        PipelineStage::Plugin,
        media_types::ErrorKind::Unsupported {
            format: Some(media_types::error::FormatInfo {
                name: Some(kind.to_owned()),
                sniffed: Vec::new(),
            }),
        },
        media_types::error::ErrorContext::new("plugin.metadata.lookup-not-served"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A provider that reads `k=v;` pairs from the probe window.
    struct PairProvider;

    impl MetadataProvider for PairProvider {
        fn extract(&mut self, probe_window: &SharedBytes) -> Result<Vec<Metadata>, EngineError> {
            let text = String::from_utf8_lossy(probe_window.as_slice()).into_owned();
            let mut out = Vec::new();
            for pair in text.split(';') {
                let pair = pair.trim();
                if pair.is_empty() {
                    continue;
                }
                match pair.split_once('=') {
                    Some((k, v)) if !v.is_empty() => out.push(Metadata::Tag(Tag {
                        name: k.to_owned(),
                        value: v.to_owned(),
                    })),
                    _ => {
                        return Err(EngineError::new(
                            PipelineStage::Probe,
                            media_types::ErrorKind::Corrupt {
                                detail: media_types::error::CorruptDetail::InvalidStructure {
                                    offset: None,
                                },
                            },
                            media_types::error::ErrorContext::new("plugin.metadata.pair-syntax"),
                        ));
                    }
                }
            }
            Ok(out)
        }
        fn provides(&self) -> Vec<&'static str> {
            vec!["tags"]
        }
        fn lookup(
            &mut self,
            kind: &str,
            _query: &BTreeMap<String, String>,
        ) -> Result<Vec<Metadata>, EngineError> {
            if kind == "tags" {
                Ok(Vec::new())
            } else {
                Err(lookup_not_served(kind))
            }
        }
    }

    #[test]
    fn extraction_reads_pairs() {
        let mut provider = PairProvider;
        let window = SharedBytes::from_static(b"title=Nightdrive;artist=Kraftwerk;");
        let found = match provider.extract(&window) {
            Ok(m) => m,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(found.len(), 2);
        match &found[0] {
            Metadata::Tag(t) => {
                assert_eq!(t.name, "title");
                assert_eq!(t.value, "Nightdrive");
            }
            _ => unreachable!("pair provider yields tags"),
        }
    }

    #[test]
    fn absence_is_data_and_damage_is_error() {
        let mut provider = PairProvider;
        // Absence: an empty probe window is Ok(vec![]), not an error.
        let none = match provider.extract(&SharedBytes::from_static(b"")) {
            Ok(m) => m,
            Err(e) => unreachable!("{e}"),
        };
        assert!(none.is_empty());
        // Damage: a pair without '=' fails structurally.
        let bad = provider.extract(&SharedBytes::from_static(b"broken"));
        assert!(bad.is_err());
    }

    #[test]
    fn lookup_gates_on_served_kinds() {
        let mut provider = PairProvider;
        assert_eq!(provider.provides(), vec!["tags"]);
        let ok = match provider.lookup("tags", &BTreeMap::new()) {
            Ok(m) => m,
            Err(e) => unreachable!("{e}"),
        };
        assert!(ok.is_empty());
        let err = match provider.lookup("lyrics", &BTreeMap::new()) {
            Ok(_) => unreachable!("lyrics not served"),
            Err(e) => e,
        };
        assert_eq!(err.stage, PipelineStage::Plugin);
        assert!(matches!(
            err.kind,
            media_types::ErrorKind::Unsupported { .. }
        ));
    }
}
