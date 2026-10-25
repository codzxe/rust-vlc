//! Playlist vocabulary (cards 4.D1 + 5.O1) — deliberately minimal and
//! verbatim.
//!
//! MP §8 / card rule: **no invented metadata.** Every field is either taken
//! verbatim from the source M3U or is absent. Durations are kept even when
//! negative (`-1` is the extended-M3U "unknown/live" marker — preserved as
//! written, not re-interpreted), titles are kept exactly as they appeared
//! after the first comma, and unknown `#EXT…` directives are preserved as
//! ordered `(name, value)` pairs so writing is lossless for everything the
//! parser understands.
//!
//! Card 5.O1 adds the HLS media-segment vocabulary ([`HlsSegment`],
//! [`HlsKey`], [`HlsMap`], [`ByteRange`]): attributes from the
//! `#EXT-X-*` subset are *recorded, not resolved* — a key names its method
//! and URI, a byte-range is kept exactly as written, and nothing is fetched,
//! decrypted, or cross-checked against segment bytes.

/// A parsed playlist. Empty playlists are legal (empty file, header-only
/// file, comments-only file).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Playlist {
    /// Entries in source order.
    pub entries: Vec<Entry>,
    /// `#EXT-X-ENDLIST` was present (HLS, card 5.O1): the playlist asserts
    /// no further segments will be appended. Recorded verbatim as a flag;
    /// the parser attaches no streaming semantics to it.
    pub end_list: bool,
}

impl Playlist {
    /// Creates an empty playlist.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether `#EXT-X-ENDLIST` was seen (convenience for
    /// `!self.end_list` inverted reads).
    #[must_use]
    pub fn is_live(&self) -> bool {
        !self.end_list
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
    /// unknown `#EXT…` tag — in source order. Unknown `#EXT-X-*` tags
    /// (e.g. `EXT-X-TARGETDURATION`) land here too (5.O1: preserved
    /// verbatim; only the typed subset is promoted).
    pub directives: Vec<(String, String)>,
    /// HLS media-segment attributes for this entry (card 5.O1): the
    /// `#EXT-X-BYTERANGE`/`-DISCONTINUITY`/`-KEY`/`-MAP` subset, recorded
    /// (not resolved). Stateful tags (`KEY`, `MAP`) are stamped on every
    /// segment they govern, so `entry.hls.key` is the key in effect *for
    /// this segment*.
    pub hls: HlsSegment,
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
            hls: HlsSegment::default(),
        }
    }
}

/// HLS media-segment attributes recorded for one entry (card 5.O1).
///
/// Everything here is **recorded, not resolved**: a key names its
/// encryption method and URI exactly as written; a byte-range offset is
/// kept as written (an absent `@o` stays `None` — the HLS "continues the
/// previous range" rule is documented, not applied); nothing is fetched
/// or decrypted. Parsing (not scheduling) is the whole of card 5.O1 —
/// the streaming scheduler is Phase 6.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HlsSegment {
    /// `#EXT-X-BYTERANGE:<n>[@<o>]` in effect for this segment.
    pub byte_range: Option<ByteRange>,
    /// `#EXT-X-DISCONTINUITY` appeared before this segment's URI line.
    pub discontinuity: bool,
    /// `#EXT-X-KEY` in effect for this segment (recorded, not resolved).
    /// `None` means no key / `METHOD=NONE`.
    pub key: Option<HlsKey>,
    /// `#EXT-X-MAP` in effect for this segment (recorded, not resolved).
    pub map: Option<HlsMap>,
}

/// `#EXT-X-BYTERANGE:<n>[@<o>]` — byte length and optional offset, exactly
/// as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByteRange {
    /// Segment length in bytes.
    pub length: u64,
    /// Starting offset into the media file. `None` when the tag omits
    /// `@o` — per the HLS spec the range then continues the previous
    /// segment's range; that rule is the consumer's to apply (recorded,
    /// not resolved).
    pub offset: Option<u64>,
}

/// `#EXT-X-KEY` attributes, recorded verbatim (card 5.O1: method + URI —
/// recorded, not resolved). `METHOD=NONE` is modeled by the *absence* of
/// a key (`entry.hls.key == None`), so this type always carries a real
/// method.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HlsKey {
    /// Encryption `METHOD` (enumerated value).
    pub method: EncryptionMethod,
    /// Key URI (quoted-string in the source).
    pub uri: String,
    /// `IV` hexadecimal-sequence, when present.
    pub iv: Option<Vec<u8>>,
    /// `KEYFORMAT` quoted-string, when present.
    pub keyformat: Option<String>,
    /// `KEYFORMATVERSIONS` quoted-string, when present.
    pub keyformat_versions: Option<String>,
}

/// `EXT-X-KEY` `METHOD` values (HLS enumerated strings).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncryptionMethod {
    /// `AES-128`.
    Aes128,
    /// `SAMPLE-AES`.
    SampleAes,
    /// Any other enumerated value, kept verbatim (forward-compatible —
    /// the HLS spec allows new methods; unknown ones are recorded, not
    /// rejected).
    Other(String),
}

/// `#EXT-X-MAP` attributes, recorded verbatim (card 5.O1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HlsMap {
    /// Init-section URI (quoted-string in the source).
    pub uri: String,
    /// `BYTERANGE` attribute (a quoted `"<n>[@<o>]"` in the source),
    /// when present.
    pub byte_range: Option<ByteRange>,
}
