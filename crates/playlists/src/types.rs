//! Playlist vocabulary (card 4.D1) — deliberately minimal and verbatim.
//!
//! MP §8 / card rule: **no invented metadata.** Every field is either taken
//! verbatim from the source M3U or is absent. Durations are kept even when
//! negative (`-1` is the extended-M3U "unknown/live" marker — preserved as
//! written, not re-interpreted), titles are kept exactly as they appeared
//! after the first comma, and unknown `#EXT…` directives are preserved as
//! ordered `(name, value)` pairs so writing is lossless for everything the
//! parser understands.

/// A parsed playlist. Empty playlists are legal (empty file, header-only
/// file, comments-only file).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Playlist {
    /// Entries in source order.
    pub entries: Vec<Entry>,
}

impl Playlist {
    /// Creates an empty playlist.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends an entry.
    pub fn push(&mut self, entry: Entry) {
        self.entries.push(entry);
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the playlist has no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Serializes to canonical extended M3U ([`crate::write`]);
    /// `parse(write(p)) == p` holds for every `p` produced by
    /// [`crate::parse`].
    #[must_use]
    pub fn to_m3u_string(&self) -> String {
        crate::write(self)
    }
}

/// One playlist entry: a media location plus whatever metadata the source
/// attached to it.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// Media location **verbatim** (after line-level whitespace trimming):
    /// an absolute/relative filesystem path (Windows or Unix separators,
    /// unresolved and un-normalized) or a URL. Never resolved against a
    /// base here — that is the caller's policy.
    pub location: String,
    /// `#EXTINF` duration in seconds, when present and finite. Negative
    /// values (e.g. the `-1` unknown marker) are preserved as written.
    pub duration: Option<f64>,
    /// `#EXTINF` title (the text after the first comma), when non-empty.
    pub title: Option<String>,
    /// Other `#EXT…:value` directives seen between the previous location
    /// line (or start of file) and this entry's location line — e.g.
    /// `("EXTVLCOPT", "http-user-agent=…")`, `("EXTGRP", "…")`, or any
    /// unknown `#EXT…` tag — in source order.
    pub directives: Vec<(String, String)>,
}

impl Entry {
    /// Creates a bare entry with no metadata.
    #[must_use]
    pub fn new(location: impl Into<String>) -> Self {
        Self {
            location: location.into(),
            duration: None,
            title: None,
            directives: Vec::new(),
        }
    }
}
