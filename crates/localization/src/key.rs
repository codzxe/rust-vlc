//! Message keys — the stable address of every user-visible string
//! (doc 07 §6.1).
//!
//! A key is 2–5 dotted segments, each `[a-z0-9-]+`. Because
//! [`MessageKey`] is a validated newtype and the translation API accepts
//! nothing else, an invalid or raw string can never reach a catalog lookup
//! (doc 07 §6.5 "deny by construction").

use std::fmt;

/// Minimum number of dotted segments in a key (doc 07 §6.1: "two to five
/// segments").
pub const MIN_SEGMENTS: usize = 2;

/// Maximum number of dotted segments in a key (doc 07 §6.1: "two to five
/// segments"). NOTE: the `msgid` pattern in doc 07 §3.2
/// (`^[a-z0-9-]+(\.[a-z0-9-]+)+$`) has no upper bound — the prose grammar
/// here is stricter; reconciling the two is a Lead change-control item.
pub const MAX_SEGMENTS: usize = 5;

/// Reserved top-level namespaces (doc 07 §6.1 table).
pub const RESERVED_NAMESPACES: &[&str] = &[
    "product",  // identity-driven strings rendered from config
    "window",   // window titles (idle title)
    "player",   // playback surface: controls, tooltips, states
    "settings", // area/section titles, setting labels + descriptions (D4)
    "menu",     // menu labels (doc 05 §8.3)
    "dialog",   // open-URL, keyboard shortcuts, onboarding
    "error",    // user-facing error rendering per error code (doc 05 §2.13)
    "common",   // shared vocabulary (OK/Cancel/Reset…) — only shareable ns
];

/// Ways a candidate key can violate the doc 07 §6.1 grammar.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KeyError {
    /// The key is empty.
    #[error("empty key")]
    Empty,
    /// Fewer than {MIN_SEGMENTS} or more than {MAX_SEGMENTS} dotted segments.
    #[error("expected between {MIN_SEGMENTS} and {MAX_SEGMENTS} dotted segments, found {found}")]
    SegmentCount { found: usize },
    /// A segment is empty, or contains a character outside `[a-z0-9-]`
    /// (uppercase, underscore, space, non-ASCII, …).
    #[error("segment {segment:?} must match [a-z0-9-]+ (kebab-case, no uppercase, no underscores)")]
    InvalidSegment { segment: String },
}

fn is_valid_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// A validated message key (doc 07 §6.1).
///
/// # Examples
///
/// ```
/// use localization::MessageKey;
///
/// let key = MessageKey::new("player.controls.play")?;
/// assert_eq!(key.as_str(), "player.controls.play");
/// assert_eq!(key.namespace(), "player");
/// assert_eq!(key.segment_count(), 3);
///
/// // Grammar violations are typed errors, never panics:
/// assert!(MessageKey::new("Player.Play").is_err()); // uppercase
/// assert!(MessageKey::new("a").is_err()); // one segment
/// assert!(MessageKey::new("a.b.c.d.e.f").is_err()); // six segments
/// # Ok::<(), localization::KeyError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MessageKey {
    raw: String,
}

impl MessageKey {
    /// Validate `raw` against the key grammar and build a [`MessageKey`].
    pub fn new(raw: &str) -> Result<Self, KeyError> {
        if raw.is_empty() {
            return Err(KeyError::Empty);
        }
        let segments: Vec<&str> = raw.split('.').collect();
        if segments.len() < MIN_SEGMENTS || segments.len() > MAX_SEGMENTS {
            return Err(KeyError::SegmentCount {
                found: segments.len(),
            });
        }
        for segment in &segments {
            if !is_valid_segment(segment) {
                return Err(KeyError::InvalidSegment {
                    segment: (*segment).to_string(),
                });
            }
        }
        Ok(Self {
            raw: raw.to_string(),
        })
    }

    /// The key exactly as written.
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    /// The first (namespace) segment, e.g. `"player"` for
    /// `player.controls.play`.
    pub fn namespace(&self) -> &str {
        // Invariant: non-empty key with >= 2 valid segments, so the split
        // always yields a non-empty first segment.
        self.raw.split('.').next().unwrap_or_default()
    }

    /// The number of dotted segments.
    pub fn segment_count(&self) -> usize {
        self.raw.split('.').count()
    }

    /// Whether the top-level namespace is one of the doc 07 §6.1 reserved
    /// namespaces. Informational only — validation does not reject
    /// unreserved namespaces (this crate's own seed uses `xtask`/`ci`).
    pub fn in_reserved_namespace(&self) -> bool {
        RESERVED_NAMESPACES.contains(&self.namespace())
    }
}

impl fmt::Display for MessageKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.raw)
    }
}

impl AsRef<str> for MessageKey {
    fn as_ref(&self) -> &str {
        &self.raw
    }
}

#[cfg(test)]
mod tests {
    use super::{KeyError, MessageKey, RESERVED_NAMESPACES, is_valid_segment};

    #[test]
    fn accepts_valid_keys() -> Result<(), KeyError> {
        for raw in [
            "a.b",
            "common.reset",
            "player.controls.play",
            "settings.area.playback.title",
            "settings.playback.subtitle-mode.label",
            "a.b-c.d-e.f",
            "error.decode.hw.init",
            "x2.y9",
            "xtask.gate.status.not-run",
        ] {
            let key = MessageKey::new(raw)?;
            assert_eq!(key.as_str(), raw);
        }
        Ok(())
    }

    #[test]
    fn rejects_invalid_keys_with_typed_errors() {
        let cases: &[(&str, KeyError)] = &[
            ("", KeyError::Empty),
            ("a", KeyError::SegmentCount { found: 1 }),
            ("a.b.c.d.e.f", KeyError::SegmentCount { found: 6 }),
            (
                "Player.play",
                KeyError::InvalidSegment {
                    segment: "Player".to_string(),
                },
            ),
            (
                "player_controls.play",
                KeyError::InvalidSegment {
                    segment: "player_controls".to_string(),
                },
            ),
            (
                "a..b",
                KeyError::InvalidSegment {
                    segment: String::new(),
                },
            ),
            (
                ".a.b",
                KeyError::InvalidSegment {
                    segment: String::new(),
                },
            ),
            (
                "a.b.",
                KeyError::InvalidSegment {
                    segment: String::new(),
                },
            ),
            (
                "a b.c",
                KeyError::InvalidSegment {
                    segment: "a b".to_string(),
                },
            ),
            (
                "a.bé.c",
                KeyError::InvalidSegment {
                    segment: "bé".to_string(),
                },
            ),
        ];
        for (raw, expected) in cases {
            let got = MessageKey::new(raw).err();
            assert_eq!(got.as_ref(), Some(expected), "case {raw:?}");
        }
    }

    #[test]
    fn namespace_and_segments() -> Result<(), KeyError> {
        let key = MessageKey::new("settings.area.playback.title")?;
        assert_eq!(key.namespace(), "settings");
        assert_eq!(key.segment_count(), 4);
        assert!(key.in_reserved_namespace());
        let tool = MessageKey::new("xtask.gate.result.success")?;
        assert_eq!(tool.namespace(), "xtask");
        assert!(!tool.in_reserved_namespace());
        Ok(())
    }

    #[test]
    fn reserved_namespace_table_matches_doc07() {
        assert_eq!(
            RESERVED_NAMESPACES,
            &[
                "product", "window", "player", "settings", "menu", "dialog", "error", "common"
            ]
        );
    }

    #[test]
    fn segment_charset() {
        assert!(is_valid_segment("play"));
        assert!(is_valid_segment("subtitle-mode"));
        assert!(is_valid_segment("x2"));
        assert!(!is_valid_segment(""));
        assert!(!is_valid_segment("Play"));
        assert!(!is_valid_segment("play "));
        assert!(!is_valid_segment("pläy"));
    }
}
