//! Playlist probing + bounded directory loading (card 5.O3).
//!
//! Two jobs, both I/O-free in this crate:
//!
//! 1. **[`probe`]** — classify a file as playlist / media / unknown from
//!    its extension plus a **bounded prefix** of its bytes (the caller
//!    supplies the bytes; recommended window: [`PROBE_PREFIX_BYTES`]).
//!    Content wins over extension for media (a `RIFF` named `.m3u` is
//!    media — same posture as media-probe's "the extension is a hint,
//!    not truth"); playlists require **extension + content agreement**,
//!    because their text signatures are weak (`[playlist]` can appear
//!    in prose; only the extension gate keeps the triage honest).
//! 2. **[`walk`]** — recursive directory enumeration **behind the
//!    [`DirSource`] access trait**: depth cap ([`DEFAULT_MAX_DEPTH`] 8,
//!    configurable), entry cap ([`DEFAULT_MAX_ENTRIES`] 10 000 — the
//!    bounded-enumeration AC), cycle-safe via canonical-path identity
//!    (symlink loops), and deterministic byte-wise name order.
//!
//! # Design decisions (recorded for review)
//!
//! - **Why a local trait (`DirSource`)?** The `input` crate's landed
//!   access surface (card 1.D2 / 2.A1) is byte-stream-only:
//!   `plugin_api::Access`/`SourceStream` open *sources*; there is no
//!   directory-listing trait anywhere in the workspace (checked at
//!   db85696 — `plugin_api::discovery` is network *device* discovery,
//!   a different concept). Per the card, the minimal trait is defined
//!   here and is the **R4 seam**: when the Lead lands directory access
//!   (input crate / plugin-api), this trait migrates there and
//!   `walk` keeps its algorithm unchanged. The crate therefore still
//!   performs no I/O of its own — every byte and listing arrives
//!   through caller-supplied impls (the 4.D1 posture, extended to
//!   directories), and no `std::fs` code ships in the library.
//! - **Media magic is a minimal local table** (RIFF / OggS / `ftyp`@4 /
//!   EBML / fLaC / ID3, exactly the card's list). `media-types`
//!   exposes no magic table (checked at db85696 — its only "magic" is
//!   Granlund–Montgomery division constants in `time.rs`), and
//!   `media-probe`'s signatures cover AVI/MP4 only and would drag a
//!   probe-crate dependency into playlists. Recorded seam: transport
//!   and raw elementary streams (`.ts`, `.ac3`, ADTS `.aac`, raw
//!   MPEG audio / H.264/HEVC, FLV) have **no entry** in this triage
//!   table and classify [`ProbedKind::Unknown`] — the deep format
//!   oracle is `media-probe` (card 2.A2); this probe is the cheap
//!   playlist-panel triage, and the corpus test pins today's table so
//!   any addition is a deliberate, tested change.
//! - **Bounded everywhere (D-1)**: probing scans only the caller's
//!   prefix (O(prefix)); the walk visits at most `max_entries` listed
//!   children and `max_depth` levels, so no unbounded work or growth
//!   is possible regardless of the filesystem's shape.
//! - **Cycle safety**: symlink loops are guarded by canonical-path
//!   identity from the source itself (the only layer that can resolve
//!   links). A directory whose identity cannot be established is
//!   skipped with a counted warning rather than risked (a walk that
//!   cannot canonicalize its root degrades to at most one bounded
//!   re-visit — depth and entry caps still bound everything).
//! - **Dirty-world tolerance (D-2)**: one unlistable nested directory
//!   degrades to a counted warning; only an unlistable *root* is a
//!   typed error (there is nothing to return). Non-UTF-8 names are
//!   the source's to skip (v0 limitation, matching `input`'s UTF-8
//!   policy) — the walker cannot count what it never sees.
//! - **No paths in errors (MP §12)**: [`WalkError`] names kinds only;
//!   OS detail rides the `source` chain, exactly like `input`'s error
//!   constructors.
//! - Std + `thiserror` only; no `unsafe`, per workspace lint policy.

use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

/// Recommended prefix size for [`probe`] — enough for every media
/// magic (≤ 8 bytes) and the leading lines of any playlist header.
pub const PROBE_PREFIX_BYTES: usize = 4096;

/// Default recursion depth cap for [`walk`]: directories up to 8 path
/// components below the root get their contents enumerated.
pub const DEFAULT_MAX_DEPTH: usize = 8;

/// Default entry cap for [`walk`] (bounded-enumeration AC): at most
/// 10 000 listed children are processed before the walk reports
/// truncation.
pub const DEFAULT_MAX_ENTRIES: usize = 10_000;

// ===========================================================================
// Probing
// ===========================================================================

/// What [`probe`] classified a file as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbedKind {
    /// A recognized playlist format (parseable by this crate or a
    /// future sibling parser).
    Playlist(PlaylistFormat),
    /// A media file: one of the demux container magic signatures
    /// matched (content wins over extension).
    Media,
    /// Neither signal fired. The file may still be media outside the
    /// triage table (transport streams, raw elementary streams) — the
    /// deep oracle is `media-probe`.
    Unknown,
}

/// The playlist formats the triage recognizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaylistFormat {
    /// Extended/plain M3U and HLS media playlists (`.m3u`/`.m3u8`).
    M3u,
    /// PLS (`[playlist]` INI header).
    Pls,
    /// XSPF (XML shareable playlist).
    Xspf,
    /// CUE sheet (CDRWIN).
    Cue,
}

/// Classify a file from its extension and a bounded byte prefix.
///
/// * `extension` — the file's extension, with or without the leading
///   dot, any case (`.M3U8`, `m3u` …). `None` = no extension.
/// * `prefix` — the file's leading bytes (caller-read; see
///   [`PROBE_PREFIX_BYTES`]). A short or empty prefix is legal — the
///   sniff simply has less evidence.
///
/// Policy: **media magic wins** (binary signatures are unambiguous);
/// playlists need the extension to name a known playlist format **and**
/// the content sniff for that format to confirm; everything else is
/// [`ProbedKind::Unknown`]. The function is pure — no I/O, no panics
/// on any byte input.
#[must_use]
pub fn probe(extension: Option<&str>, prefix: &[u8]) -> ProbedKind {
    if is_media_magic(prefix) {
        return ProbedKind::Media;
    }
    let Some(ext) = extension else {
        return ProbedKind::Unknown;
    };
    let ext = ext.strip_prefix('.').unwrap_or(ext);
    let confirmed = if ext.eq_ignore_ascii_case("m3u") || ext.eq_ignore_ascii_case("m3u8") {
        confirm_m3u(prefix).then_some(PlaylistFormat::M3u)
    } else if ext.eq_ignore_ascii_case("pls") {
        confirm_pls(prefix).then_some(PlaylistFormat::Pls)
    } else if ext.eq_ignore_ascii_case("xspf") {
        confirm_xspf(prefix).then_some(PlaylistFormat::Xspf)
    } else if ext.eq_ignore_ascii_case("cue") {
        confirm_cue(prefix).then_some(PlaylistFormat::Cue)
    } else {
        None
    };
    match confirmed {
        Some(format) => ProbedKind::Playlist(format),
        None => ProbedKind::Unknown,
    }
}

/// [`probe`] over a path: the extension comes from the path, the
/// prefix still arrives from the caller (this crate does no I/O).
#[must_use]
pub fn probe_path(path: &Path, prefix: &[u8]) -> ProbedKind {
    probe(path.extension().and_then(|e| e.to_str()), prefix)
}

/// The demux container magic table (the card 5.O3 list): RIFF (WAV/
/// AVI), OggS (Ogg), `ftyp` at offset 4 (MP4/MOV/M4A), EBML (MKV/
/// WebM), fLaC, ID3 (tagged MP3).
fn is_media_magic(prefix: &[u8]) -> bool {
    if prefix.len() >= 4 {
        let head = &prefix[..4];
        if head == b"RIFF" || head == b"OggS" || head == b"fLaC" {
            return true;
        }
        if head == [0x1A, 0x45, 0xDF, 0xA3] {
            return true;
        }
    }
    // ID3 (tagged MP3, or a tag stapled ahead of another stream) is a
    // 3-byte signature: `starts_with`, not the 4-byte head compare.
    if prefix.starts_with(b"ID3") {
        return true;
    }
    prefix.len() >= 8 && &prefix[4..8] == b"ftyp"
}

/// Strip a UTF-8 BOM if present.
fn strip_bom(prefix: &[u8]) -> &[u8] {
    if prefix.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &prefix[3..]
    } else {
        prefix
    }
}

/// Playlists are text: a NUL byte in the sniffed window disqualifies
/// every playlist confirmation.
fn has_nul(prefix: &[u8]) -> bool {
    prefix.contains(&0)
}

/// One trimmed line (CR + ASCII whitespace stripped at both ends).
fn next_line<'a>(rest: &mut &'a [u8]) -> Option<&'a [u8]> {
    if rest.is_empty() {
        return None;
    }
    let Some(pos) = rest.iter().position(|&b| b == b'\n') else {
        let line = *rest;
        *rest = &[];
        return Some(line.trim_ascii());
    };
    let line = &rest[..pos];
    *rest = &rest[pos + 1..];
    Some(line.trim_ascii())
}

/// M3U: an `#EXTM3U` header line, or a first non-comment,
/// non-blank line (by the format's own looseness, every such line is
/// a location — the sniff can be no stronger, and says so).
fn confirm_m3u(prefix: &[u8]) -> bool {
    if has_nul(prefix) {
        return false;
    }
    let mut rest = strip_bom(prefix);
    while let Some(line) = next_line(&mut rest) {
        if line.is_empty() {
            continue;
        }
        if line.first() == Some(&b'#') {
            if line.eq_ignore_ascii_case(b"#EXTM3U") {
                return true;
            }
            continue;
        }
        // First non-comment line: a location (URI / relative path).
        return true;
    }
    false
}

/// PLS: the first non-blank line is the `[playlist]` INI header.
fn confirm_pls(prefix: &[u8]) -> bool {
    if has_nul(prefix) {
        return false;
    }
    let mut rest = strip_bom(prefix);
    while let Some(line) = next_line(&mut rest) {
        if line.is_empty() {
            continue;
        }
        return line.eq_ignore_ascii_case(b"[playlist]");
    }
    false
}

/// XSPF: an XML declaration and a `<playlist` element, both present
/// in the prefix (case-sensitive — XML's vocabulary is).
fn confirm_xspf(prefix: &[u8]) -> bool {
    let body = strip_bom(prefix);
    contains_sub(body, b"<?xml") && contains_sub(body, b"<playlist")
}

fn contains_sub(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// CUE: a `FILE` keyword line (case-insensitive, word-boundary
/// checked) carrying a quoted filename — CDRWIN makes `FILE` the
/// mandatory first command, so requiring it is honest. `TRACK`/`REM`
/// lines alone do not confirm (`REM COMMENT "…"` would false-positive).
fn confirm_cue(prefix: &[u8]) -> bool {
    if has_nul(prefix) {
        return false;
    }
    let mut rest = strip_bom(prefix);
    while let Some(line) = next_line(&mut rest) {
        if line.len() < 4 || !line[..4].eq_ignore_ascii_case(b"file") {
            continue;
        }
        let after = &line[4..];
        let boundary = matches!(after.first(), None | Some(b' ') | Some(b'\t') | Some(b'"'));
        if !boundary {
            continue;
        }
        if after.iter().filter(|&&b| b == b'"').count() >= 2 {
            return true;
        }
    }
    false
}

// ===========================================================================
// Directory loading
// ===========================================================================

/// One immediate child of a listed directory, as reported by a
/// [`DirSource`]. The **source** resolves symlink targets when it can
/// (an fs impl classifies symlink-to-dir as [`DirEntryKind::Dir`],
/// symlink-to-file as [`DirEntryKind::File`]); the walker stays pure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedEntry {
    /// The child's base name (UTF-8; non-UTF-8 names are the source's
    /// to skip — v0 limitation, matching `input`'s policy).
    pub name: String,
    /// What the child is.
    pub kind: DirEntryKind,
}

/// What a listed child is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirEntryKind {
    /// A regular file (including symlinks the source resolved to
    /// files).
    File,
    /// A directory (including symlinks the source resolved to
    /// directories — cycle safety is the walker's job, via
    /// [`DirSource::canonical`]).
    Dir,
    /// A symlink the source could not resolve, or a special file
    /// (socket, fifo, device). Skipped, counted.
    Symlink,
    /// Anything else. Skipped, counted.
    Other,
}

/// The minimal directory-access trait (card 5.O3): list one
/// directory's immediate children, and give a directory a **stable
/// identity** for cycle detection.
///
/// No such trait exists in the landed access surface (checked at
/// db85696): `plugin_api::Access` opens byte streams;
/// `plugin_api::discovery` is network device discovery. This is the
/// **R4 seam** — when directory access lands in `input`/`plugin-api`,
/// this trait migrates there and [`walk`] is unchanged.
///
/// # Contract
///
/// - `list` returns the *complete* immediate-child set of `dir` (the
///   walker sorts; any order is fine). Following symlinked paths is
///   the source's business (fs semantics).
/// - `canonical` returns a stable, per-directory-unique path identity
///   (an fs impl: `fs::canonicalize`). Two paths with the same
///   identity are the same directory. An error means "no identity" —
///   the walker skips such directories (counted), never risks a loop.
pub trait DirSource {
    /// List the immediate children of `dir`.
    ///
    /// # Errors
    /// [`io::Error`] when the directory cannot be read (permissions,
    /// not a directory, …). For the *root* of a walk this surfaces as
    /// a typed error; for nested directories it degrades to a
    /// counted warning.
    fn list(&self, dir: &Path) -> io::Result<Vec<ListedEntry>>;

    /// A stable identity for the directory at `path` (see trait docs).
    ///
    /// # Errors
    /// [`io::Error`] when no identity can be established.
    fn canonical(&self, path: &Path) -> io::Result<PathBuf>;
}

/// Walk configuration (all bounds enforced; see [`walk`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WalkConfig {
    /// Maximum number of path components below the root whose
    /// contents are enumerated: `1` = only the root's immediate
    /// children; `8` (default) = `root/a/b/c/d/e/f/g/h/file` is
    /// collected, anything deeper is not.
    pub max_depth: usize,
    /// Maximum number of listed children processed in total (files,
    /// directories, skipped specials all count — the walk budget).
    /// Reaching it truncates the walk ([`WalkResult::truncated`]).
    pub max_entries: usize,
}

impl Default for WalkConfig {
    fn default() -> Self {
        Self {
            max_depth: DEFAULT_MAX_DEPTH,
            max_entries: DEFAULT_MAX_ENTRIES,
        }
    }
}

impl WalkConfig {
    /// Validated constructor: both bounds must be nonzero (a zero
    /// depth would enumerate nothing; a zero budget nothing at all —
    /// both are configuration bugs, surfaced typed, not clamped).
    ///
    /// # Errors
    /// [`WalkError::InvalidConfig`] when `max_depth` or `max_entries`
    /// is zero.
    pub fn new(max_depth: usize, max_entries: usize) -> Result<Self, WalkError> {
        if max_depth == 0 || max_entries == 0 {
            return Err(WalkError::InvalidConfig);
        }
        Ok(Self {
            max_depth,
            max_entries,
        })
    }
}

/// Typed walk failures. Names kinds only — no paths (MP §12); OS
/// detail rides the `source` chain.
#[derive(Debug, thiserror::Error)]
pub enum WalkError {
    /// The configuration was rejected (zero bound).
    #[error("playlists: walk config rejected (both bounds must be nonzero)")]
    InvalidConfig,

    /// The walk's root directory could not be enumerated — there is
    /// nothing to return. Nested-directory failures are *not* errors
    /// (they degrade to [`WalkWarnings::unlistable_dirs`]).
    #[error("playlists: root directory could not be enumerated")]
    RootUnlistable(#[source] io::Error),
}

/// The counted, non-fatal walk findings (D-2: one bad directory never
/// kills the walk).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WalkWarnings {
    /// Directories skipped because their canonical identity was
    /// already visited — the symlink-loop guard firing.
    pub cycles_skipped: usize,
    /// Nested directories whose listing failed.
    pub unlistable_dirs: usize,
    /// Directories skipped because no canonical identity could be
    /// established (cycle safety is not provable for them).
    pub unidentifiable_dirs: usize,
    /// Children skipped as unresolvable symlinks / special files.
    pub skipped_special: usize,
}

/// The outcome of a [`walk`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WalkResult {
    /// Every regular file found, in deterministic order: depth-first,
    /// children in byte-wise name order (no locale collation), a
    /// directory's subtree emitted at the directory's position.
    pub files: Vec<PathBuf>,
    /// The entry cap was reached: the result is a bounded prefix of
    /// the full enumeration, not the whole tree.
    pub truncated: bool,
    /// Counted non-fatal findings.
    pub warnings: WalkWarnings,
}

/// Recursively enumerate a directory tree behind a [`DirSource`],
/// bounded on both axes.
///
/// * **Depth cap** — [`WalkConfig::max_depth`] path components below
///   the root; deeper directories are seen (counted) but not
///   descended into.
/// * **Entry cap** — [`WalkConfig::max_entries`] listed children in
///   total; the walk stops and reports [`WalkResult::truncated`].
///   `files.len()` therefore never exceeds `max_entries`.
/// * **Cycle-safe** — every descended directory's canonical identity
///   (from [`DirSource::canonical`]) is remembered; a revisit is
///   skipped and counted (symlink loops cannot spin the walk, even
///   before the caps fire).
/// * **Deterministic** — children are processed in byte-wise name
///   order, so the same tree yields the same `files` order on every
///   run and platform, whatever order the source lists in.
///
/// # Errors
/// [`WalkError::RootUnlistable`] when the root itself cannot be
/// listed. Everything dirty below the root degrades to
/// [`WalkWarnings`] (D-2).
///
/// # Examples
/// ```
/// use playlists::probe::{DirEntryKind, DirSource, ListedEntry, WalkConfig, walk};
/// use std::io;
/// use std::path::{Path, PathBuf};
///
/// // A one-directory in-memory source (test/conformance shape).
/// struct OneDir;
/// impl DirSource for OneDir {
///     fn list(&self, dir: &Path) -> io::Result<Vec<ListedEntry>> {
///         if dir == Path::new("/lib") {
///             Ok(vec![
///                 ListedEntry { name: "a.mp3".into(), kind: DirEntryKind::File },
///                 ListedEntry { name: "sub".into(), kind: DirEntryKind::Dir },
///             ])
///         } else if dir == Path::new("/lib/sub") {
///             Ok(vec![ListedEntry { name: "b.flac".into(), kind: DirEntryKind::File }])
///         } else {
///             Err(io::Error::new(io::ErrorKind::NotFound, "no such dir"))
///         }
///     }
///     fn canonical(&self, path: &Path) -> io::Result<PathBuf> {
///         Ok(path.to_path_buf()) // identity — real impls canonicalize
///     }
/// }
///
/// let cfg = WalkConfig::default();
/// let result = walk(&OneDir, Path::new("/lib"), &cfg)
///     .unwrap_or_else(|e| unreachable!("{e}"));
/// assert_eq!(result.files, [PathBuf::from("/lib/a.mp3"), PathBuf::from("/lib/sub/b.flac")]);
/// assert!(!result.truncated);
/// ```
pub fn walk(
    source: &dyn DirSource,
    root: &Path,
    config: &WalkConfig,
) -> Result<WalkResult, WalkError> {
    let mut visited: HashSet<PathBuf> = HashSet::new();
    let mut warnings = WalkWarnings::default();
    let mut files: Vec<PathBuf> = Vec::new();
    let mut truncated = false;
    let mut seen: usize = 0;

    // The root's identity anchors the loop guard. Without it (an
    // unidentifiable root) a top-level link back to the root costs at
    // most one bounded re-visit — the caps still bound the walk.
    match source.canonical(root) {
        Ok(id) => {
            visited.insert(id);
        }
        Err(_) => warnings.unidentifiable_dirs += 1,
    }

    // Iterative depth-first walk with an explicit work stack (no
    // recursion — `max_depth` is caller-configured and must never map
    // to call-stack depth). Each popped item is either "emit this
    // file" or "expand this directory"; a directory's children are
    // pushed in reverse name order so the smallest pops first — the
    // emitted order is pre-order DFS, byte-wise name order, with a
    // directory's subtree at the directory's position.
    enum Work {
        List { dir: PathBuf, depth: usize },
        File { path: PathBuf },
    }
    let mut stack: Vec<Work> = vec![Work::List {
        dir: root.to_path_buf(),
        depth: 0,
    }];

    while let Some(work) = stack.pop() {
        let (dir, depth) = match work {
            Work::File { path } => {
                files.push(path);
                continue;
            }
            Work::List { dir, depth } => (dir, depth),
        };
        let mut listing = match source.list(&dir) {
            Ok(listing) => listing,
            Err(e) => {
                if depth == 0 {
                    return Err(WalkError::RootUnlistable(e));
                }
                warnings.unlistable_dirs += 1;
                continue;
            }
        };
        // Deterministic order regardless of the source's listing order.
        listing.sort_by(|a, b| a.name.cmp(&b.name));

        // Children are accounted in sorted order (so truncation keeps
        // the sorted-order prefix), then pushed in reverse — the
        // smallest pops first.
        let mut expand: Vec<Work> = Vec::new();
        for entry in listing {
            if seen == config.max_entries {
                truncated = true;
                break;
            }
            seen += 1;
            let child = dir.join(&entry.name);
            match entry.kind {
                DirEntryKind::File => expand.push(Work::File { path: child }),
                DirEntryKind::Dir => {
                    // Depth cap: a child at depth+1 components is
                    // descended into only while more levels remain
                    // inside max_depth (files at exactly max_depth
                    // components are still collected).
                    if depth + 1 < config.max_depth {
                        match source.canonical(&child) {
                            Ok(id) => {
                                if !visited.insert(id) {
                                    warnings.cycles_skipped += 1;
                                    continue;
                                }
                            }
                            Err(_) => {
                                warnings.unidentifiable_dirs += 1;
                                continue;
                            }
                        }
                        expand.push(Work::List {
                            dir: child,
                            depth: depth + 1,
                        });
                    }
                }
                DirEntryKind::Symlink | DirEntryKind::Other => {
                    warnings.skipped_special += 1;
                }
            }
        }
        stack.extend(expand.into_iter().rev());
        if truncated {
            // Budget spent: emit every file already enumerated (the
            // stack's not-yet-popped tail, in order) and stop — no
            // further directory is expanded.
            while let Some(work) = stack.pop() {
                if let Work::File { path } = work {
                    files.push(path);
                }
            }
            break;
        }
    }

    Ok(WalkResult {
        files,
        truncated,
        warnings,
    })
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::Read;

    // -----------------------------------------------------------------
    // Probing: the classification table
    // -----------------------------------------------------------------

    #[test]
    fn probe_table_playlist_formats() {
        let m3u_ext = b"#EXTM3U\n#EXTINF:120,Title\nsong.mp3\nhttp://x/y\n";
        assert_eq!(
            probe(Some("m3u"), m3u_ext),
            ProbedKind::Playlist(PlaylistFormat::M3u)
        );
        assert_eq!(
            probe(Some(".M3U8"), m3u_ext),
            ProbedKind::Playlist(PlaylistFormat::M3u)
        );
        // Plain M3U: no header, first non-comment line is a location.
        let plain = b"\n# a comment\n\n  ../rel/path file.mp3\n";
        assert_eq!(
            probe(Some("m3u8"), plain),
            ProbedKind::Playlist(PlaylistFormat::M3u)
        );

        let pls = b"[playlist]\nFile1=x.mp3\nNumberOfEntries=1\nVersion=2\n";
        assert_eq!(
            probe(Some("pls"), pls),
            ProbedKind::Playlist(PlaylistFormat::Pls)
        );

        let xspf = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<playlist version=\"1\" xmlns=\"http://xspf.org/ns/0/\">\n";
        assert_eq!(
            probe(Some("xspf"), xspf),
            ProbedKind::Playlist(PlaylistFormat::Xspf)
        );

        let cue = b"REM GENRE \"Test\"\nFILE \"track01.wav\" WAVE\n  TRACK 01 AUDIO\n";
        assert_eq!(
            probe(Some("cue"), cue),
            ProbedKind::Playlist(PlaylistFormat::Cue)
        );
        // BOM + case tolerance on the FILE keyword.
        let cue_bom = b"\xEF\xBB\xBFfile \"t.wav\" MP3\n";
        assert_eq!(
            probe(Some("cue"), cue_bom),
            ProbedKind::Playlist(PlaylistFormat::Cue)
        );
    }

    #[test]
    fn probe_table_playlist_negatives() {
        // Extension names a playlist, content disagrees -> Unknown.
        assert_eq!(
            probe(Some("m3u"), b"\x00\x01\x02binary"),
            ProbedKind::Unknown
        );
        assert_eq!(
            probe(Some("m3u"), b"#comment only\n#EXTINF:1\n"),
            ProbedKind::Unknown
        );
        assert_eq!(probe(Some("m3u"), b""), ProbedKind::Unknown);
        assert_eq!(probe(Some("pls"), b"File1=x.mp3\n"), ProbedKind::Unknown); // no [playlist] header
        assert_eq!(
            probe(Some("xspf"), b"<?xml version=\"1.0\"?>\n<rss/>\n"),
            ProbedKind::Unknown
        );
        // REM/TRACK without a quoted-filename FILE line is not a cue.
        assert_eq!(
            probe(Some("cue"), b"REM COMMENT \"x\"\n  TRACK 01 AUDIO\n"),
            ProbedKind::Unknown
        );
        // filename.wav must not read as a FILE keyword (boundary check).
        assert_eq!(probe(Some("cue"), b"filename \"x\"\n"), ProbedKind::Unknown);
        // Extension gate: prose containing [playlist] is not a playlist.
        assert_eq!(
            probe(Some("txt"), b"[playlist]\nFile1=lie\n"),
            ProbedKind::Unknown
        );
        // No extension, no magic.
        assert_eq!(probe(None, b"random text\n"), ProbedKind::Unknown);
    }

    #[test]
    fn probe_table_media_magic_wins() {
        let mp4 = b"\x00\x00\x00\x20ftypisom\x00\x00\x02\x00isom";
        for (ext, bytes) in [
            ("avi", b"RIFF\x24\x08\x00\x00AVI LIST" as &[u8]),
            ("wav", b"RIFF\xb8\x62\x00\x00WAVEfmt "),
            ("mkv", b"\x1A\x45\xDF\xA3\x93\x42\x82\x88matroska"),
            ("ogg", b"OggS\x00\x02\x00\x00\x00\x00\x00\x00\x00\x00"),
            ("flac", b"fLaC\x00\x00\x00\x22\x10\x00\x10\x40"),
            ("mp4", mp4),
            ("mp3", b"ID3\x04\x00\x00\x00\x00\x00\x00tagged"),
        ] {
            assert_eq!(probe(Some(ext), bytes), ProbedKind::Media, "ext {ext}");
            // And with a *wrong* extension, and with none: content wins.
            assert_eq!(
                probe(Some("m3u"), bytes),
                ProbedKind::Media,
                "m3u-named {ext}"
            );
            assert_eq!(probe(None, bytes), ProbedKind::Media, "no-ext {ext}");
        }
        // ftyp needs all 8 leading bytes: a shorter window has no verdict.
        assert_eq!(
            probe(Some("mp4"), b"\x00\x00\x00\x20fty"),
            ProbedKind::Unknown
        );
        assert_eq!(probe(Some("mp4"), b"free text file"), ProbedKind::Unknown);
    }

    #[test]
    fn probe_path_uses_the_path_extension() {
        let path = Path::new("/media/MyMix.M3U8");
        assert_eq!(
            probe_path(path, b"#EXTM3U\na.mp3\n"),
            ProbedKind::Playlist(PlaylistFormat::M3u)
        );
        assert_eq!(
            probe_path(Path::new("/x/noext"), b"junk"),
            ProbedKind::Unknown
        );
    }

    /// The playlist-vs-media classification table over the real corpus
    /// (D-11: no synthetic echo chamber). Runs when the corpus is
    /// present; prints a skip note otherwise (CI portability).
    #[test]
    fn classification_table_over_real_corpus() {
        let media_dir = Path::new("/tmp/my-project/corpus-v4/media");
        let mut names: Vec<_> = match std::fs::read_dir(media_dir) {
            Ok(iter) => iter
                .filter_map(|e| e.ok())
                .map(|e| e.file_name())
                .filter_map(|n| n.into_string().ok())
                .collect(),
            Err(_) => {
                eprintln!("corpus not present — skipping (run on the dev machine)");
                return;
            }
        };
        names.sort();
        assert!(
            names.len() >= 120,
            "expected the real corpus, got {} files",
            names.len()
        );

        let (mut media, mut unknown) = (0usize, 0usize);
        let mut families: HashMap<&'static str, usize> = HashMap::new();
        for name in &names {
            let path = media_dir.join(name);
            let prefix = read_prefix(&path);
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            // The expected-kind table (documents today's triage):
            // container families are Media by magic; transport and raw
            // elementary streams are Unknown (outside the card's magic
            // list — recorded seam; the deep oracle is media-probe).
            let expected = match ext.as_str() {
                "avi" | "wav" => ProbedKind::Media,          // RIFF
                "mkv" | "mka" | "webm" => ProbedKind::Media, // EBML
                "mp4" | "m4a" | "mov" => ProbedKind::Media,  // ftyp
                "ogg" | "ogv" => ProbedKind::Media,          // OggS
                "flac" => ProbedKind::Media,                 // fLaC
                "mp3" => {
                    // Tagged MP3 (ID3) is Media; raw MPEG audio has no
                    // entry in the triage table -> Unknown (pinned seam).
                    if prefix.starts_with(b"ID3") {
                        ProbedKind::Media
                    } else {
                        ProbedKind::Unknown
                    }
                }
                "ts" | "mp2" | "aac" | "h264" | "265" | "ac3" | "flv" => ProbedKind::Unknown,
                other => unreachable!("corpus extension not in the table: {other}"),
            };
            let got = probe_path(&path, &prefix);
            assert_eq!(got, expected, "{name}");
            match got {
                ProbedKind::Media => media += 1,
                ProbedKind::Unknown => unknown += 1,
                ProbedKind::Playlist(_) => unreachable!("corpus media files are not playlists"),
            }
            *families
                .entry(Box::leak(ext.clone().into_boxed_str()))
                .or_insert(0) += 1;
        }
        // Every magic family in the corpus was actually exercised.
        for family in ["avi", "wav", "mkv", "webm", "mp4", "mov", "ogg", "flac"] {
            assert!(
                families.get(family).copied().unwrap_or(0) > 0,
                "family {family} untested"
            );
        }
        eprintln!(
            "corpus triage: {} files — {media} Media / {unknown} Unknown (raw/transport seam)",
            names.len()
        );
        assert!(media >= 100);
        assert!(unknown >= 5);
    }

    /// Real HLS playlists from the corpus (master + variant shapes,
    /// both TS-segmented and fMP4 styles).
    #[test]
    fn real_hls_playlists_classify_as_m3u() {
        let hls_root = Path::new("/tmp/my-project/corpus-v4/hls");
        let Ok(streams) = std::fs::read_dir(hls_root) else {
            eprintln!("hls corpus not present — skipping");
            return;
        };
        let mut paths: Vec<PathBuf> = Vec::new();
        for stream in streams {
            let stream = stream.unwrap_or_else(|e| unreachable!("{e}")).path();
            let Ok(files) = std::fs::read_dir(&stream) else {
                continue;
            };
            for file in files {
                let file = file.unwrap_or_else(|e| unreachable!("{e}")).path();
                if file.extension().is_some_and(|e| e == "m3u8") {
                    paths.push(file);
                }
            }
        }
        assert!(paths.len() >= 5, "expected the real HLS playlists");
        for path in &paths {
            let prefix = read_prefix(path);
            assert_eq!(
                probe_path(path, &prefix),
                ProbedKind::Playlist(PlaylistFormat::M3u),
                "{}",
                path.display()
            );
        }
        eprintln!("hls playlists checked: {}", paths.len());
    }

    fn read_prefix(path: &Path) -> Vec<u8> {
        let mut file = File::open(path).unwrap_or_else(|e| unreachable!("{e}"));
        let mut buf = vec![0u8; PROBE_PREFIX_BYTES];
        let got = file.read(&mut buf).unwrap_or_else(|e| unreachable!("{e}"));
        buf.truncate(got);
        buf
    }

    // -----------------------------------------------------------------
    // Walking: the fake DirSource
    // -----------------------------------------------------------------

    /// In-memory filesystem: listings keyed by canonical identity,
    /// plus an explicit link table (a real fs resolves links inside
    /// `canonicalize`; the fake models them as path -> identity).
    #[derive(Default)]
    struct FakeFs {
        listing: HashMap<PathBuf, Vec<ListedEntry>>,
        canon: HashMap<PathBuf, PathBuf>,
        fail_list: HashSet<PathBuf>,
        fail_canon: HashSet<PathBuf>,
        reverse: bool,
    }

    impl FakeFs {
        fn dir(&mut self, path: &str, children: &[(&str, DirEntryKind)]) -> &mut Self {
            let mut entries: Vec<ListedEntry> = children
                .iter()
                .map(|(name, kind)| ListedEntry {
                    name: (*name).to_owned(),
                    kind: *kind,
                })
                .collect();
            if self.reverse {
                entries.reverse();
            }
            self.listing.insert(PathBuf::from(path), entries);
            self
        }
        fn link(&mut self, from: &str, to: &str) -> &mut Self {
            self.canon.insert(PathBuf::from(from), PathBuf::from(to));
            self
        }
        fn fail(&mut self, path: &str) -> &mut Self {
            self.fail_list.insert(PathBuf::from(path));
            self
        }
        fn fail_canon(&mut self, path: &str) -> &mut Self {
            self.fail_canon.insert(PathBuf::from(path));
            self
        }
        fn resolve(&self, p: &Path) -> PathBuf {
            self.canon
                .get(p)
                .cloned()
                .unwrap_or_else(|| p.to_path_buf())
        }
    }

    impl DirSource for FakeFs {
        fn list(&self, dir: &Path) -> io::Result<Vec<ListedEntry>> {
            let id = self.resolve(dir);
            if self.fail_list.contains(&id) {
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "scripted"));
            }
            match self.listing.get(&id) {
                Some(entries) => Ok(entries.clone()),
                // A directory with no registered listing does not exist
                // (an unknown root must fail typed, not walk empty).
                None => Err(io::Error::new(io::ErrorKind::NotFound, "no such directory")),
            }
        }
        fn canonical(&self, path: &Path) -> io::Result<PathBuf> {
            if self.fail_canon.contains(path) {
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "scripted"));
            }
            Ok(self.resolve(path))
        }
    }

    #[test]
    fn nested_directories_in_deterministic_dfs_order() {
        let mut fs = FakeFs::default();
        fs.dir(
            "/r",
            &[
                ("b", DirEntryKind::Dir),
                ("c.txt", DirEntryKind::File),
                ("a", DirEntryKind::Dir),
                ("z2.txt", DirEntryKind::File),
                ("sock", DirEntryKind::Other),
            ],
        )
        .dir(
            "/r/a",
            &[("k", DirEntryKind::Dir), ("m.txt", DirEntryKind::File)],
        )
        .dir("/r/a/k", &[("x.txt", DirEntryKind::File)])
        .dir("/r/b", &[("z1.txt", DirEntryKind::File)]);

        let cfg = WalkConfig::default();
        let result = walk(&fs, Path::new("/r"), &cfg).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(
            result.files,
            [
                PathBuf::from("/r/a/k/x.txt"),
                PathBuf::from("/r/a/m.txt"),
                PathBuf::from("/r/b/z1.txt"),
                PathBuf::from("/r/c.txt"),
                PathBuf::from("/r/z2.txt"),
            ]
        );
        assert!(!result.truncated);
        assert_eq!(
            result.warnings.skipped_special, 1,
            "the socket is counted, skipped"
        );
        assert_eq!(result.warnings.cycles_skipped, 0);

        // Same tree, source lists in reverse: identical output (the
        // walk's own sort is the determinism, not the source's).
        let mut rev = FakeFs {
            reverse: true,
            ..FakeFs::default()
        };
        rev.dir(
            "/r",
            &[
                ("b", DirEntryKind::Dir),
                ("c.txt", DirEntryKind::File),
                ("a", DirEntryKind::Dir),
                ("z2.txt", DirEntryKind::File),
            ],
        )
        .dir(
            "/r/a",
            &[("k", DirEntryKind::Dir), ("m.txt", DirEntryKind::File)],
        )
        .dir("/r/a/k", &[("x.txt", DirEntryKind::File)])
        .dir("/r/b", &[("z1.txt", DirEntryKind::File)]);
        let again = walk(&rev, Path::new("/r"), &cfg).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(result.files, again.files);
    }

    #[test]
    fn depth_cap_is_enforced() {
        // A 12-deep chain with a marker file at every level: f<N> sits
        // exactly N components below the root (/r/d1/…/d{N-1}/f<N>.txt).
        let mut fs = FakeFs::default();
        fs.dir(
            "/r",
            &[("d1", DirEntryKind::Dir), ("f1.txt", DirEntryKind::File)],
        );
        let mut dir = String::from("/r/d1");
        for n in 2..=12usize {
            fs.dir(
                &dir,
                &[
                    (
                        Box::leak(format!("d{}", n).into_boxed_str()),
                        DirEntryKind::Dir,
                    ),
                    (
                        Box::leak(format!("f{n}.txt").into_boxed_str()),
                        DirEntryKind::File,
                    ),
                ],
            );
            dir = format!("{dir}/d{n}");
        }

        // max_depth = 4: files up to 4 components below /r — f4.txt is
        // in (exactly 4), f5.txt is out (5).
        let cfg = WalkConfig::new(4, 10_000).unwrap_or_else(|e| unreachable!("{e}"));
        let result = walk(&fs, Path::new("/r"), &cfg).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(
            result.files,
            [
                PathBuf::from("/r/d1/d2/d3/f4.txt"),
                PathBuf::from("/r/d1/d2/f3.txt"),
                PathBuf::from("/r/d1/f2.txt"),
                PathBuf::from("/r/f1.txt"),
            ]
        );
        assert!(!result.truncated);
        assert_eq!(
            result.warnings,
            WalkWarnings::default(),
            "no spurious findings"
        );

        // Default depth 8 over the same 12-chain: f8.txt in, f9.txt out.
        let deep = walk(&fs, Path::new("/r"), &WalkConfig::default())
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(deep.files.len(), 8);
        assert!(
            deep.files
                .contains(&PathBuf::from("/r/d1/d2/d3/d4/d5/d6/d7/f8.txt"))
        );
        assert!(
            !deep
                .files
                .iter()
                .any(|p| p.to_string_lossy().contains("f9.txt"))
        );

        // max_depth = 1: only the root's immediate children.
        let shallow = WalkConfig::new(1, 10_000).unwrap_or_else(|e| unreachable!("{e}"));
        let result = walk(&fs, Path::new("/r"), &shallow).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(result.files, [PathBuf::from("/r/f1.txt")]);
    }

    #[test]
    fn symlink_cycle_is_detected_and_bounded() {
        let mut fs = FakeFs::default();
        fs.dir(
            "/r",
            &[
                ("loop", DirEntryKind::Dir), // -> /r itself
                ("sub", DirEntryKind::Dir),
                ("keep.txt", DirEntryKind::File),
            ],
        )
        .dir(
            "/r/sub",
            &[("back", DirEntryKind::Dir), ("s.txt", DirEntryKind::File)],
        ) // back -> /r
        .link("/r/loop", "/r")
        .link("/r/sub/back", "/r");

        let result = walk(&fs, Path::new("/r"), &WalkConfig::default())
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(
            result.warnings.cycles_skipped, 2,
            "both loop links are caught"
        );
        assert_eq!(
            result.files,
            [PathBuf::from("/r/keep.txt"), PathBuf::from("/r/sub/s.txt")]
        );
        assert!(!result.truncated);
    }

    #[test]
    fn entry_cap_truncates_the_walk() {
        let mut fs = FakeFs::default();
        let children: Vec<(Box<str>, DirEntryKind)> = (0..15)
            .map(|n| (format!("f{n:02}.txt").into_boxed_str(), DirEntryKind::File))
            .collect();
        let refs: Vec<(&str, DirEntryKind)> = children.iter().map(|(n, k)| (&**n, *k)).collect();
        fs.dir("/r", &refs);

        let cfg = WalkConfig::new(8, 10).unwrap_or_else(|e| unreachable!("{e}"));
        let result = walk(&fs, Path::new("/r"), &cfg).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(result.files.len(), 10, "exactly the budget");
        assert!(result.truncated, "the cap is reported");
        assert_eq!(result.files[0], PathBuf::from("/r/f00.txt"));
        assert_eq!(result.files[9], PathBuf::from("/r/f09.txt"));
        // The default budget is the AC's 10k.
        assert_eq!(WalkConfig::default().max_entries, DEFAULT_MAX_ENTRIES);
        assert_eq!(WalkConfig::default().max_depth, DEFAULT_MAX_DEPTH);
    }

    #[test]
    fn root_unlistable_is_typed_nested_degrades_to_warning() {
        let mut fs = FakeFs::default();
        fs.dir(
            "/r",
            &[("bad", DirEntryKind::Dir), ("good.txt", DirEntryKind::File)],
        )
        .dir("/r/bad", &[])
        .fail("/r/bad");

        let result = walk(&fs, Path::new("/r"), &WalkConfig::default())
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(result.files, [PathBuf::from("/r/good.txt")]);
        assert_eq!(result.warnings.unlistable_dirs, 1);
        assert!(!result.truncated);

        let err = match walk(&fs, Path::new("/missing"), &WalkConfig::default()) {
            Err(e) => e,
            Ok(_) => unreachable!("an unlistable root must fail typed"),
        };
        assert!(matches!(err, WalkError::RootUnlistable(_)));
        assert!(err.to_string().contains("root directory"));
    }

    #[test]
    fn config_zero_bounds_are_rejected_typed() {
        assert!(matches!(
            WalkConfig::new(0, 10),
            Err(WalkError::InvalidConfig)
        ));
        assert!(matches!(
            WalkConfig::new(2, 0),
            Err(WalkError::InvalidConfig)
        ));
        let cfg = WalkConfig::new(2, 5).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!((cfg.max_depth, cfg.max_entries), (2, 5));
    }

    /// Unidentifiable directories (no canonical identity available)
    /// are skipped and counted — the walk never risks an unprovable
    /// cycle. A dir with no children but a healthy identity is simply
    /// empty (no warning).
    #[test]
    fn unidentifiable_directory_is_skipped_and_counted() {
        let mut fs = FakeFs::default();
        fs.dir(
            "/r",
            &[
                ("ghost", DirEntryKind::Dir),
                ("empty", DirEntryKind::Dir),
                ("f.txt", DirEntryKind::File),
            ],
        )
        .dir("/r/empty", &[])
        .dir("/r/ghost", &[("never.txt", DirEntryKind::File)])
        .fail_canon("/r/ghost");

        let result = walk(&fs, Path::new("/r"), &WalkConfig::default())
            .unwrap_or_else(|e| unreachable!("{e}"));
        // ghost is never descended into: its file is absent.
        assert_eq!(result.files, [PathBuf::from("/r/f.txt")]);
        assert_eq!(result.warnings.unidentifiable_dirs, 1);
        // The empty dir has an identity and no children: no warning.
        assert_eq!(result.warnings.unlistable_dirs, 0);
        assert_eq!(result.warnings.cycles_skipped, 0);
    }

    /// Probing integrates with walking: classify every walked file.
    #[test]
    fn walked_files_classify_through_probe() {
        let mut fs = FakeFs::default();
        fs.dir(
            "/r",
            &[
                ("mix.m3u", DirEntryKind::File),
                ("song.flac", DirEntryKind::File),
                ("cover.jpg", DirEntryKind::File),
                ("sub", DirEntryKind::Dir),
            ],
        )
        .dir("/r/sub", &[("movie.mkv", DirEntryKind::File)]);

        let result = walk(&fs, Path::new("/r"), &WalkConfig::default())
            .unwrap_or_else(|e| unreachable!("{e}"));
        // Byte-wise name order: cover.jpg < mix.m3u < song.flac < sub/…
        let m3u = b"#EXTM3U\n#EXTINF:1,t\nsub/movie.mkv\n";
        let flac = b"fLaC\x00\x00\x00\x22";
        let mkv = b"\x1A\x45\xDF\xA3\x93\x42\x82\x88matroska";
        assert_eq!(result.files[0], PathBuf::from("/r/cover.jpg"));
        assert_eq!(
            probe_path(&result.files[0], b"\xFF\xD8\xFF\xE0jpeg"),
            ProbedKind::Unknown
        );
        assert_eq!(result.files[1], PathBuf::from("/r/mix.m3u"));
        assert_eq!(
            probe_path(&result.files[1], m3u),
            ProbedKind::Playlist(PlaylistFormat::M3u)
        );
        assert_eq!(result.files[2], PathBuf::from("/r/song.flac"));
        assert_eq!(probe_path(&result.files[2], flac), ProbedKind::Media);
        assert_eq!(result.files[3], PathBuf::from("/r/sub/movie.mkv"));
        assert_eq!(probe_path(&result.files[3], mkv), ProbedKind::Media);
    }
}
