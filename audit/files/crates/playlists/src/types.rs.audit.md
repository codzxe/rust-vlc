# AUDIT: crates/playlists/src/types.rs
| Field | Value |
|---|---|
| Auditor | W10-A09 |
| Date | 2026-11-21 |
| Lines / functions | 186 / 15 items (2 structs + 6 methods, 3 structs + 1 ctor, 3 vocab types, module docs) |
| File verdict | REAL (pure data vocabulary) / UNREACHABLE from shipped execution paths (compiled into the shipped dep graph via streaming→playlists, but no shipped lane constructs these types) |
| Reachable from shipped entry point? | NO for execution — the shipped CLI's streaming usage (fetch/ranged/schemes) imports nothing from playlists; the crates' consumers are streaming's hls/dash modules (themselves unreachable — see my hls.rs#part1 audit), pls/xspf/m3u/cue parsers, and tests. The types are LINKED into `lumen` (playlists is a streaming dep) but no call path from `main` reaches them. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg "is_live\(\)|to_m3u_string|Playlist::new|Entry::new" **/*.rs` (callers: tests + sibling parsers only — no production caller of `is_live` at all); read of types.rs:1-186 complete; cross-read m3u.rs:569-610 (`parse_ext_x_key` — the double-`Option` key-clearing that backs this file's `METHOD=NONE` doc claim); cross-read hls.rs:46 (ByteRange/HlsKey/HlsMap consumers); baseline: playlists tests never ran (established this session) |

## 1. One-paragraph verdict
A clean, honest data vocabulary: every field verbatim-or-absent, the `-1` unknown-duration marker preserved rather than reinterpreted, stateful HLS attributes (KEY/MAP) stamped per-segment with the parser actually implementing the stamping and the `METHOD=NONE` clearing this file documents (verified end-to-end against `parse_ext_x_key`'s double-`Option` — the doc claim is true). The single defect is a dead, misnamed convenience: `Playlist::is_live()` answers `!end_list`, which is true for **every plain M3U** (no ENDLIST tag exists outside HLS), yet the method reads as a media-type predicate; it has zero production callers. Everything else was checked and holds.

## 2. What actually works (verified)
— `METHOD=NONE` doc claim (:148-150 "modeled by the *absence* of a key") verified END-TO-END: m3u.rs:587-592 returns `Some(None)` for a URI-less NONE (clearing the stateful key) and treats NONE-with-URI as damage — the type invariant "HlsKey always carries a real method" is upheld by the only constructor path.
— `to_m3u_string` (:65-67) delegates to `crate::write`; its roundtrip law is correctly SCOPED to parse-produced playlists ("holds for every `p` produced by [`crate::parse`]" :62-63) — a hand-built `Entry` whose location contains a newline would not roundtrip, and the doc does not claim it would.
— `Playlist::len`/`is_empty` pair present (clippy len-without-is-empty satisfied); `new()`/`Default` consistent; `#[must_use]` on all pure queries.
— `ByteRange.offset: Option<u64>` models the absent-`@o` "continues previous range" form explicitly, with the resolution rule documented as the consumer's (:140-144) — matches hls.rs:97-98's identical posture.
— `EncryptionMethod::Other(String)` forward-compat for new HLS methods — recorded, not rejected; consistent with the parser's match (m3u.rs:596-599).
— No logic, no arithmetic, no panics, no `unsafe`, no feature gates — a vocabulary file that does exactly what its docs say.

## 3. Findings

### F-001 [S4]
Where: crates/playlists/src/types.rs:37-42 (`Playlist::is_live`)
Code:
```
    /// Whether `#EXT-X-ENDLIST` was seen (convenience for
    /// `!self.end_list` inverted reads).
    #[must_use]
    pub fn is_live(&self) -> bool {
        !self.end_list
    }
```
What is wrong: two defects in one method. (a) Dead: `rg` shows the only caller in the repository is a test (tests/hls_attrs.rs:108) — no production code path calls it; the streaming HLS code reads `parsed.playlist.end_list` directly (hls.rs:315) instead. (b) Misleading name: `end_list` is false for **every playlist that is not HLS** — a plain `.m3u` or a PLS/XSPF bridged into this vocabulary answers `is_live() == true`, importing an HLS-only concept ("live") onto all playlist kinds. The doc admits the mechanics ("convenience for `!self.end_list` inverted reads") but the name promises a media predicate.
Why it matters: the next consumer will reach for `is_live()` and get a wrong answer for every non-HLS playlist; dead misnamed API is exactly the kind of seam that gets wired wrong later.
What correct looks like: `has_end_list()`/`is_ended()` (mechanical name), or move the method to the HLS model where ENDLIST has meaning and delete it here.
Confidence: HIGH — Verification: read + rg caller sweep (header).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-16 | REAL | — | "No invented metadata" contract — upheld by every field (all `Option`, all verbatim). |
| `Playlist` (struct) | 20-28 | REAL | — | Plain data; empty-playlist legality documented and true. |
| `Playlist::new` | 30-35 | REAL | — | Default-consistent. |
| `Playlist::is_live` | 37-42 | REAL | S4 | F-001: dead + misnamed for non-HLS kinds. |
| `Playlist::push` | 44-47 | REAL | — | Trivial. |
| `Playlist::len` | 51-53 | REAL | — | Trivial. |
| `Playlist::is_empty` | 55-59 | REAL | — | Trivial. |
| `Playlist::to_m3u_string` | 61-67 | REAL | — | Delegates to `write`; roundtrip law correctly scoped to parse-produced values (reasoned: hand-built newline locations don't roundtrip, doc doesn't claim they do). |
| `Entry` (struct) | 72-97 | REAL | — | Directives ordered-pairs preservation is the lossless-write contract (consumed by xspf/pls bridges). |
| `Entry::new` | 99-111 | REAL | — | Trivial ctor. |
| `HlsSegment` | 121-132 | REAL | — | Recorded-not-resolved; stamping semantics verified against the parser. |
| `ByteRange` | 136-145 | REAL | — | Absent-offset modeled; consumer rule documented. |
| `HlsKey` | 151-163 | REAL | — | Always-real-method invariant upheld by parser (verified). |
| `EncryptionMethod` | 166-176 | REAL | — | Forward-compat Other(String). |
| `HlsMap` | 179-186 | REAL | — | Plain data. |

## 5. Test honesty
No tests in this file. Cross-file pins (not mine to audit, referenced): tests/m3u_suite.rs:356-358 (`to_m3u_string == write`, empty write = `#EXTM3U\n`), tests/hls_attrs.rs:108 (`is_live` false after ENDLIST) and :497-538 (KEY/MAP stamping). None executed in the committed baseline (playlists test binary never ran).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "`METHOD=NONE` is modeled by the absence of a key" | :148-150 | REAL | m3u.rs:587-592 returns `Some(None)` (state cleared); NONE-with-URI is damage. |
| "parse(write(p)) == p holds for every p produced by crate::parse" | :61-63 | REAL-UNPROVEN | Correctly scoped; pinned by m3u_suite tests that never ran in baseline. |
| "nothing is fetched, decrypted, or cross-checked against segment bytes" | :14-16, :115-120 | REAL | The type carries strings/bytes only; no I/O anywhere in the crate. |

## 7. Dead / duplicated / unreachable code
`is_live` dead (F-001). Whole file unreachable from shipped execution paths (header) though linked via the streaming dep.

## 8. Dependencies & flags
None (std only, no thiserror even — vocabulary only). No feature gates.

## 9. Open questions for the next agent
- Whoever audits m3u.rs: confirm the KEY-stamping reset semantics on `#EXT-X-DISCONTINUITY`-adjacent state (the per-segment stamping I verified covers NONE-clearing; discontinuity does not clear keys per spec — check the parser agrees).
