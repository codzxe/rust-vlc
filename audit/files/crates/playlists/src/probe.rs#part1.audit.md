# AUDIT: crates/playlists/src/probe.rs#part1 (lines 1-617)
| Field | Value |
|---|---|
| Auditor | W10-A09 |
| Date | 2026-11-21 |
| Lines / functions | 1-617 (chunk boundary declared: part1 = 1-617, ALL production code; part2 = 618-1234, the `#[cfg(test)]` module [A10]) / 27 items |
| File verdict | UNREACHABLE (real, careful code; zero production callers — re-exported from playlists/src/lib.rs:115-119, consumed by nothing) |
| Reachable from shipped entry point? | NO — `rg` over all `**/*.rs`: `probe`/`probe_path`/`walk`/`DirSource` appear only in probe.rs itself, lib.rs re-export (:115-119), and the module's own tests/doctest. `streaming` (the only crate depending on playlists besides fuzz) uses only `playlists::{hls,mpd}` types (hls/media.rs:86, hls/master.rs:76, hls/live.rs:94, dash/*). No `apps/` crate depends on playlists. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 0 |
| Commands run | `rg "playlists::" apps/` (nothing); `rg "playlists = \{ path" **/Cargo.toml` (only fuzz + streaming); `rg "DirSource|WalkConfig|ListedEntry|probe::walk|DirEntryKind" **/*.rs` (probe.rs + lib.rs only); `rg "probe|walk|DirSource" crates/streaming/src` (no matches); `rg "probe|walk" crates/playlists/tests` (no matches); `ls fuzz/fuzz_targets` + fuzz/Cargo.toml (no probe/walk target); read of probe.rs:1-630; `sed -n` quote verifies at 190-194, 252-255, 359-362, 566-595; baseline: playlists unit tests NEVER RAN (both baseline runs abort before the playlists test binary — test-workspace.log.txt dies at apps/desktop, test-workspace-allfeatures.log.txt dies at first-vertical evidence) |

## 1. One-paragraph verdict
The code is honest, careful, panic-free triage + bounded-walk machinery — every claim in its module docs is implemented except two: the XSPF sniffer skips the NUL-disqualification rule the module's own `has_nul` doc says applies to "every playlist confirmation", and the `WalkConfig::max_depth` doc example is off by one level versus the actual descent predicate. The dominating fact is reachability: none of this is callable from the shipped binary — no app depends on `playlists`, and the one production dependent (`streaming`) touches only the HLS/MPD models. The "playlist panel triage" this module exists to serve does not ship, so 617 lines of pure, tested code are dead weight in the build until a UI/queue consumer lands.

## 2. What actually works (verified)
— `probe` (128-151) traced by hand: `.m3u`+`#EXTM3U` → Playlist(M3u); `.m3u`+prose-first-line → Playlist(M3u) (documented looseness :211-213); `.txt`+`#EXTM3U` → Unknown (extension gate); `RIFF…` named `.m3u` → Media (content wins, doc :8-10 honored); `None` extension → Media or Unknown only.
— `is_media_magic` (163-179): all six signatures guarded against short prefixes (`len >= 4` head compare, `len >= 8` for `ftyp`@4, `starts_with` for 3-byte ID3) — no index panic on any byte input; traced with 0/3/4/7/8-byte prefixes.
— `next_line` (197-209): `rest=[b'\n']` → Some(empty) then None; no infinite loop; `\r` stripped via `trim_ascii`.
— `walk` (497-616): iterative DFS with explicit stack (no recursion — correct given caller-configured depth); ordering traced by hand on a two-child root (file "a.mp3" + dir "sub" → a.mp3, sub/b.flac, matching the doctest :491-495; reversed-push/pop gives pre-order with subtree at directory position); `seen`/`max_entries` invariant holds (`files.len() ≤ max_entries` — the cap check precedes each child); root-unlistable is the only typed error (:545-546), nested failures degrade to counted warnings (:548-549) exactly as documented; cycle guard anchors on the root's canonical identity (:511-516) and inserts every descended dir's identity before expansion (:574-585).
— `WalkConfig::new` (387-395): zero bounds refused typed, never clamped.

## 3. Findings

### F-001 [S3]
Where: crates/playlists/src/probe.rs:252-255 (`confirm_xspf`) vs :190-194 (`has_nul` doc)
Code:
```
fn confirm_xspf(prefix: &[u8]) -> bool {
    let body = strip_bom(prefix);
    contains_sub(body, b"<?xml") && contains_sub(body, b"<playlist")
}
```
and the rule it violates (`:190-191`):
```
/// Playlists are text: a NUL byte in the sniffed window disqualifies
/// every playlist confirmation.
```
What is wrong: `confirm_m3u` (:215), `confirm_pls` (:238) and `confirm_cue` (:267) all gate on `has_nul`; `confirm_xspf` does not. A binary blob named `.xspf` containing the two ASCII substrings anywhere in the first 4096 bytes confirms as `Playlist(Xspf)`.
Why it matters: the module's own stated invariant ("every playlist confirmation") is false for one of the four formats — a binary false-positive is exactly the confusion the NUL rule exists to prevent, and the doc/code pair now disagrees about the module's contract. Downstream the XSPF parser would damage-control it, so blast radius is bounded (and the module is unreachable today — see F-003), but the invariant is broken as written.
What correct looks like: `if has_nul(prefix) { return false; }` at the top of `confirm_xspf` (one line), matching its three siblings.
Confidence: HIGH — Verification: read (:190-194, :214-284 all four confirmers compared).

### F-002 [S3]
Where: crates/playlists/src/probe.rs:359-363 (`WalkConfig::max_depth` doc) vs :571-573 (descent predicate)
Code (doc, :359-362):
```
    /// Maximum number of path components below the root whose
    /// contents are enumerated: `1` = only the root's immediate
    /// children; `8` (default) = `root/a/b/c/d/e/f/g/h/file` is
    /// collected, anything deeper is not.
```
Code (predicate, :573):
```
                    if depth + 1 < config.max_depth {
```
What is wrong: the example path has **nine** components below the root (a,b,c,d,e,f,g,h,file). The predicate descends into a child dir only while `depth+1 < max_depth`, so with `max_depth = 8` directories at depths 1..=7 are enumerated and the deepest collectable file is `root/a/b/c/d/e/f/g/file` (eight components). The doc promises one more level than the code delivers. The inline comment at :570-572 ("files at exactly max_depth components are still collected") states the true semantics; the field doc contradicts it.
Why it matters: `max_depth` is a public configuration knob with an arithmetic contract; an integrator sizing the cap from the doc example silently loses the deepest directory level. The doctest and `depth_cap_is_enforced` (:1013, part2) pin the code, not the doc.
What correct looks like: doc example `root/a/b/c/d/e/f/g/file` (8 components), or restate the invariant as "directories up to `max_depth - 1` levels below the root are enumerated; files up to `max_depth` levels are collected".
Confidence: HIGH — Verification: read + hand-simulation of the predicate for max_depth ∈ {1, 2, 8} against the doc examples.

### F-003 [S3]
Where: crates/playlists/src/probe.rs:127-158, 339-354, 497-616 (the module's public surface)
Code (lib.rs:115-119, the only wiring):
```
pub use probe::{
    DEFAULT_MAX_DEPTH, DEFAULT_MAX_ENTRIES, DirEntryKind, DirSource, ListedEntry,
    PROBE_PREFIX_BYTES, PlaylistFormat, ProbedKind, WalkConfig, WalkError, WalkResult,
    WalkWarnings, probe, probe_path, walk,
};
```
What is wrong: the entire card-5.O3 surface — `probe`, `probe_path`, `walk`, `DirSource`, and their config/result types — has zero production callers. `apps/` does not depend on `playlists`; `streaming` imports only the HLS/MPD models; the fuzz crate's five playlist targets parse M3U/PLS/XSPF/CUE/MPD but never probe/walk; `crates/playlists/tests/*` never touch it. The "R4 seam" (`DirSource`, :21-32) waits for a directory-access trait that has not landed, and the consuming "playlist panel" (lib.rs:5) has not landed either.
Why it matters: 617 lines of dead-in-shipped code (plus its ~600-line test module) that the docs describe as the triage layer for a panel no shipped surface opens — classic "implemented ahead of the consumer, consumer never came". Nothing false is *claimed* about it (the crate docs say the panel is "upcoming"), but the wave's reachability question has a hard NO answer for the whole module.
What correct looks like: either land the consumer (queue/panel card) or mark the module `#[allow(dead_code)]`-equivalent / gate it behind the feature the panel will carry, so the shipped surface visibly matches what is reachable.
Confidence: HIGH — Verification: rg sweeps listed in the header; Cargo.toml dependency graph read (playlists' dependents: fuzz [dev], streaming [prod, HLS/MPD types only]).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-63 | REAL | — | Accurate except the two doc defects in F-001/F-002; the "checked at db85696" provenance notes are code-archaeology claims, not verifiable here (UNVERIFIED, no git in session-2). |
| `PROBE_PREFIX_BYTES` | 69-71 | REAL | — | 4096. |
| `DEFAULT_MAX_DEPTH` / `DEFAULT_MAX_ENTRIES` | 73-80 | REAL | — | 8 / 10 000. |
| `ProbedKind` | 87-99 | REAL | — | Enum. |
| `PlaylistFormat` | 101-112 | REAL | — | Enum: M3u/Pls/Xspf/Cue. |
| `probe` | 127-151 | REAL | — | Extension gate + content confirm; traced 6 cases by hand. |
| `probe_path` | 155-158 | REAL | — | Thin, correct. |
| `is_media_magic` | 163-179 | REAL | — | Panic-free on all lengths; 6 signatures. |
| `strip_bom` | 182-188 | REAL | — | Correct. |
| `has_nul` | 192-194 | REAL | — | Correct; doc overpromise = F-001. |
| `next_line` | 197-209 | REAL | — | Traced edge cases; no loop. |
| `confirm_m3u` | 214-233 | REAL | — | Documented looseness (first non-comment line confirms). |
| `confirm_pls` | 236-248 | REAL | — | Correct. |
| `confirm_xspf` | 252-255 | REAL | S3 | F-001 (no NUL gate). |
| `contains_sub` | 257-259 | REAL | — | O(n·m) on ≤4096 bytes — fine. |
| `confirm_cue` | 265-284 | REAL | — | Word-boundary + ≥2 quotes; traced `filename` (rejected) and `FILE "x" WAVE` (accepted). |
| `ListedEntry` / `DirEntryKind` | 294-318 | REAL | — | Plain data. |
| `DirSource` (trait) | 339-354 | REAL | — | The R4 seam; no impl outside tests. |
| `WalkConfig` + `Default` + `new` | 356-396 | REAL | S3 | F-002 (doc off-by-one); validation correct. |
| `WalkError` | 398-411 | REAL | — | No paths in errors (MP §12) — honored. |
| `WalkWarnings` | 413-427 | REAL | — | Four counters. |
| `WalkResult` | 429-441 | REAL | — | Plain data. |
| `walk` | 497-616 | REAL | — | Bounded, cycle-safe, deterministic; truncation drain drops pending `Work::List` silently but flags `truncated` — defensible; no panic path found (all slicing guarded). |

## 5. Test honesty (tests live in part2 :618-1234 [A10]; baseline NEVER RAN them)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `probe_table_playlist_formats` (:634) | YES | Extension+content matrix pins `probe`'s four arms. |
| `probe_table_media_magic_wins` (:710) | YES | Content-over-extension contract. |
| `classification_table_over_real_corpus` (:755) | YES | Real fixture files through `probe` — would catch magic-table regressions. |
| `depth_cap_is_enforced` (:1013) | YES | Pins the real depth predicate (and thereby the F-002 doc divergence). |
| `symlink_cycle_is_detected_and_bounded` (:1081) | YES | Cycle guard. |
| `entry_cap_truncates_the_walk` (:1112) | YES | Budget invariant. |
| **Execution** | **NEVER RAN** | Both baseline `cargo test` runs abort before the `playlists` test binary (fail-fast at apps/desktop / first-vertical). Doctest included. Green-in-CI is not in the committed evidence. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Content wins over extension for media" | module doc :8-10 | REAL | probe :129-131 read. |
| "playlists require extension + content agreement" | module doc :10-12 | SHALLOW | True for M3U/PLS/CUE; XSPF needs only two substrings (still extension-gated, but no line structure) — acceptable, but see F-001 for the NUL rule. |
| "a NUL byte … disqualifies every playlist confirmation" | :190-191 | FALSE | F-001: `confirm_xspf` has no NUL gate. |
| "`8` (default) = `root/a/b/c/d/e/f/g/h/file` is collected" | :361-362 | FALSE | F-002: predicate collects ≤ 8 components. |
| "cycle-safe via canonical-path identity" | :16-17, :449-455 | REAL | :511-516, :574-585 read. |
| "deterministic byte-wise name order" | :17, :456-458 | REAL | :553 sort + reversed-push pop order traced by hand. |

## 7. Dead / duplicated / unreachable code
The whole module — see F-003. `DirSource` has no implementation outside the test double (:885-946) and the doctest's `OneDir`.

## 8. Dependencies & flags
`std` + `thiserror` only (WalkError derive). No feature gates. Crate dep graph: playlists ← streaming (HLS/MPD types only) + fuzz (dev).

## 9. Open questions for the next agent
- A10 (part2): does any test pin `confirm_xspf` against a NUL-bearing prefix (would F-001 be caught by an existing test)? The name list suggests no.
- Wave 12+ (UI/queue wave): if the playlist panel lands, F-003 flips to reachable — re-audit then.
