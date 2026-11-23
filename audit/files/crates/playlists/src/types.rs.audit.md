# AUDIT: crates/playlists/src/types.rs
| Field | Value |
|---|---|
| Auditor | W10-A05 |
| Date | 2026-11-23 |
| Lines / functions | 186 / 8 production items (Playlist×6, Entry::new, 5 data types/enums) — no in-file tests |
| File verdict | REAL (pure, honest data model) but effectively UNREACHABLE from shipped hosts |
| Reachable from shipped entry point? | NO in practice — proof below (only streaming consumer of the parse→types path is the unreachable remote_playlist) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg for consumers (streaming/ apps/), od -c byte check, baseline citation |

## 1. One-paragraph verdict
A clean, deliberately minimal data vocabulary: `Playlist`/`Entry` carry M3U metadata verbatim ("no invented metadata" is actually honored — durations preserved as written incl. `-1`, unknown `#EXT…` directives preserved as ordered pairs, HLS attributes "recorded, not resolved" with the consumer's responsibilities documented per field). There is no logic here to break: 6 trivial accessors and one delegating serializer. Two structural problems sit above the file: (1) its only production consumer chain is `parse_bytes`→`Playlist`/`Entry` from streaming's remote_playlist.rs — which is itself unreachable — while the shipped CLI does not depend on the playlists crate at all, and streaming's HLS session uses a DIFFERENT, parallel vocabulary (hls.rs `MediaPlaylist`, per hls/media.rs:86); so this model is exercised only by tests; (2) `is_live()` answers "true" for every plain M3U (no `#EXT-X-ENDLIST` tag exists in M3U), a nonsense answer for the majority of playlists this type models. Note: my Bash tool's output sanitizer displays `#[must_use]` as `#ust_use]`; I byte-verified with `od -c` that the file contains the correct `#[must_use]` — that is a tooling artifact of MY session, not a repo defect.

## 2. What actually works (verified)
— Baseline audit/runs/baseline/cargo-test-workspace.txt (PACKAGE playlists): lib `test result: ok. 46 passed; 0 failed` — types.rs's model is exercised end-to-end by m3u_suite (18), hls_attrs (20), cue/pls/xspf suites (48 more). Byte-verified field layout of the roundtrip claim's anchor lines with `od -c`.

## 3. Findings
F-001 [S3]
Where: crates/playlists/src/types.rs:21-28 (the model) + repo-wide consumer search
Code:
```
pub struct Playlist {
    /// Entries in source order.
    pub entries: Vec<Entry>,
```
What is wrong: production reachability is empty. `rg 'playlists::(Entry|Playlist|parse_bytes|parse_detailed|write)\b' crates/streaming/src` (non-test) yields exactly one hit: remote_playlist.rs:178 `let parsed = playlists::parse_bytes(&response.body)...` — and remote_playlist is itself unreachable (its own audit, F-001). apps/cli does not list `playlists` in its Cargo.toml. The streaming HLS session uses hls.rs's `MediaPlaylist` (hls/media.rs:86 `use playlists::{HlsPlaylistError, MediaPlaylist};`), not this type. Why it matters: the M3U vocabulary — parse, write, roundtrip, the whole card 4.D1/5.O1 surface — is a test-only island in the shipped product; its 86 passing tests are not evidence of any user-visible playlist feature. What correct looks like: wire local/remote playlist open into the CLI, or mark the vocabulary test-support. Confidence: HIGH — Verification: ran the rg commands.

F-002 [S4]
Where: crates/playlists/src/types.rs:37-42
Code:
```
    /// Whether `#EXT-X-ENDLIST` was seen (convenience for
    /// `!self.end_list` inverted reads).
    #[must_use]
    pub fn is_live(&self) -> bool {
        !self.end_list
    }
```
What is wrong: `is_live()` is true for every playlist that lacks an HLS tag — i.e., every plain/extended M3U, PLS-shaped content modeled through this type, and header-only playlists. "Live" is an HLS concept; applying this accessor to the M3U majority yields a nonsense `true`. The doc string half-admits it ("convenience for inverted reads") but the name lies for most inputs. Blast radius: today nothing shipped calls it (F-001); the hls_attrs VOD fixture asserts `!p.is_live()` correctly. Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/playlists/src/types.rs:113-132 (HlsSegment family) vs crates/playlists/src/hls.rs:54 (MediaPlaylist)
Code:
```
pub struct HlsSegment {
    /// `#EXT-X-BYTERANGE:<n>[@<o>]` in effect for this segment.
    pub byte_range: Option<ByteRange>,
```
What is wrong: the crate carries TWO parallel HLS vocabularies for the same `#EXT-X-*` tags — this M3U-side subset (card 5.O1) and hls.rs's full `MediaPlaylist` (which streaming uses). Attribute semantics (key statefulness, byterange continuation) are modeled twice; divergence between the two models is possible and uncheckable by one test suite. Also cross-ref: the writer fed by this model emits `#EXTINF` without the RFC 8216 mandatory comma (root cause m3u.rs:370-377, filed in the hls_attrs.rs audit F-001 — this file's `to_m3u_string` is the delegating surface). Confidence: HIGH — Verification: read both files.

## 4. Per-function table (mandatory, >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Playlist::new | 31-35 | REAL | — | Default |
| Playlist::is_live | 37-42 | REAL | S4 | misleading name for non-HLS playlists (F-002) |
| Playlist::push / len / is_empty | 44-59 | REAL | — | trivial |
| Playlist::to_m3u_string | 61-67 | REAL | — | delegates to crate::write; roundtrip claim correctly scoped "for every p produced by crate::parse" |
| Entry::new | 100-110 | REAL | — | bare entry, hls default |
| HlsSegment (type) | 121-132 | REAL | S4 | parallel vocabulary (F-003); "recorded, not resolved" doctrine honest |
| ByteRange (type) | 136-145 | REAL | — | offset None = continuation rule left to consumer, documented |
| HlsKey / EncryptionMethod / HlsMap (types) | 151-186 | REAL | — | METHOD=NONE modeled as absence, documented; Other(String) forward-compat |

## 5. Test honesty
No tests in this file (correct — pure data). Coverage via the suite files: hls_attrs.rs 20 tests assert exact model contents against ffmpeg fixtures and RFC vectors (see its audit); m3u/cue/pls/xspf suites assert parse/write behavior on this model.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no invented metadata. Every field is either taken verbatim from the source M3U or is absent" | types.rs:4-6 | REAL | field-by-field read; ffmpeg-fixture tests pin verbatim values (hls_attrs.rs:88-92) |
| "attributes from the #EXT-X-* subset are recorded, not resolved — nothing is fetched, decrypted, or cross-checked" | types.rs:12-16 | REAL | no I/O anywhere in the model; consumers' duties documented per field |
| "parse(write(p)) == p holds for every p produced by crate::parse" | types.rs:62-63 | REAL (correctly scoped) | assert_roundtrip in hls_attrs enforces exactly this law |
| "the streaming scheduler is Phase 6" | types.rs:120 | REAL (scheduler exists in streaming but is unwired from hosts — see remote_playlist/mockhttp audits) | streaming/src/hls exists; apps/cli never calls it |

## 7. Dead / duplicated / unreachable code
Unreachable-in-product: the whole M3U vocabulary (F-001). Duplication: parallel HLS vocabularies (F-003). Within the file: nothing dead.

## 8. Dependencies & flags
std only (no imports beyond crate::write). No features, no unsafe. Byte-verified `[must_use]` attributes present (od -c).

## 9. Open questions for the next agent
- hls.rs (MediaPlaylist/MasterPlaylist) auditor: confirm whether the M3U-side HLS subset (this file) and hls.rs can ever disagree on KEY/MAP/BYTERANGE semantics, and which one the (unwired) streaming scheduler trusts.
- If a feature-matrix row claims M3U/XSPF/PLS/CUE playlist SUPPORT in the product, cite F-001: parse/write exist and are well-tested, but no shipped host opens any playlist.
