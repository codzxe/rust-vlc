# AUDIT: crates/playback-engine/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W03-A10 |
| Date | 2026-11-21 |
| Lines / functions | 145 / 15 module decls + 33 re-export groups |
| File verdict | REAL (doc-governance defects only) |
| Reachable from shipped entry point? | YES — apps/cli/Cargo.toml depends on playback-engine; every baseline smoke run (`lumen open`, avi-firstvertical-open-session.txt, sample-420 plays) executed this crate |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `awk` count of `Transition {` in TRANSITIONS (371-665) → 33; `sed -n '970,974p' state.rs` → in-file test asserts 33; `rg "TRANSITIONS.len"` → table.rs:132-133 (dynamic), model_check.rs:312 (33); `cat Cargo.toml` → 6 internal path deps, 0 external; `rg -c unsafe src/` → 1 match = the doc claim itself; `ls tests/` → all referenced suites exist; fixture `avi_mjpeg_2pcm.avi` verified in MANIFEST.toml:24 |

## 1. One-paragraph verdict
The crate root is an honest architecture map with two stale numbers in it: the module tour, conventions, and acceptance-criteria pointers all check out against real files and green test suites, "zero external dependencies / no unsafe" is literally true, and the crate is the shipped engine's core. But the front-door doc asserts the transition table has "26 rows" (line 9) and "29 legal rows, 75 typed rejections" (line 63) while the actual `TRANSITIONS` table has 33 rows — a count pinned by the crate's own in-file test (state.rs:972) and model_check.rs:312, so the crate simultaneously documents 26, 29, and 33 for the same table. Three pub modules (bookmarks, chapters, snapshot) are missing from the documented tour, and the Cargo.toml header still describes the pre-3.A1 scope ("Seek/sync are deliberately EXCLUDED").

## 2. What actually works (verified)
- "Std-only, zero external dependencies, no unsafe" (88): TRUE — [dependencies] is exactly 6 workspace-internal path crates (codec-api, commands, media-clock, media-types, packet-queue, plugin-api), zero registry crates; the single `unsafe` text match in src/ is this doc claim itself (rg -c).
- "The 8 wire-visible states" (7): TRUE — SessionState has exactly 8 variants (state.rs:86-107, counted).
- The 8 × 13 = 104 classification frame: TRUE — tests/model_check.rs:254 asserts `table.len() == 104`; table.rs:132-133 derives legal/illegal dynamically from `TRANSITIONS.len()` (so the TEST is correct even though the doc number is stale).
- Every acceptance-criteria suite named in the doc exists: tests/table.rs, tests/session_flow.rs, tests/seek_flow.rs, tests/track_switch.rs, tests/model_check.rs (ls of tests/), plus the deeper suites (contract_suite, scenarios_b, soak, property). All green in `orchrun-test-workspace-nofailfast.log.txt` (playback-engine absent from the 7 failures — the suite totals include its ~40 binaries).
- The real multi-track fixture claim (84): `avi_mjpeg_2pcm.avi` exists (tests/fixtures/generated/MANIFEST.toml:24; its structure.json shows 1 video + 2 audio interleaves — genuinely multi-track).
- Re-exports compile against real items (baseline build #1 CLEAN).

## 3. Findings

### F-001 [S3]
Where: crates/playback-engine/src/lib.rs:9-10 and 63
Code:
```rust
//! and the transition table as **data** ([`TRANSITIONS`] — 26 rows,
//! every one a legal doc 05 §3.3 wire transition). Around it:
```
and
```rust
//!   `(state, input)` pair (8 × 13 = 104: 29 legal rows, 75 typed
//!   rejections), proves every row is wire-conformant against
```
What is wrong: both stated counts are false. The actual `TRANSITIONS` table (state.rs:371-665) contains 33 rows — a number pinned by the crate's own in-file test (`assert_eq!(TRANSITIONS.len(), 33)`, state.rs:972) and by tests/model_check.rs:312. The doc says 26 (line 9) and 29 legal/75 rejected (line 63; the true split is 33 legal / 71 rejected).
Why it matters: this is the crate's front door — the document every auditor, contributor, and doc-wave agent reads first to size the state machine. Three different numbers (26, 29, 33) for one table across two lines of the same file is exactly the doc-drift the audit exists to catch; a reader cross-checking against doc 05's transition count would conclude the table over- or under-implements by 4–7 rows.
What correct looks like: "33 rows" at line 9 and "33 legal rows, 71 typed rejections" at line 63 — or better, stop hard-coding counts in prose and point at the asserting test.
Confidence: HIGH — Verification: awk row count over the table range; both asserting tests read.

### F-002 [S4]
Where: crates/playback-engine/src/lib.rs:93-107 (declarations) vs the tour at 12-47
Code:
```rust
pub mod bookmarks;
pub mod chapters;
pub mod epoch;
```
What is wrong: the "What this crate is" module tour documents 12 modules but the crate declares 15 pub modules — `bookmarks`, `chapters`, and `snapshot` have no tour entry, no one-line purpose statement, while smaller siblings (error, frame_step) do.
Why it matters: the tour is the map; three undocumented pub modules mean the crate's advertised surface is narrower than its real surface (bookmarks/chapters carry their own re-exported functions at 109-111).
What correct looks like: three more bullets in the tour.
Confidence: HIGH — Verification: diff of tour names vs `pub mod` declarations.

### F-003 [S4]
Where: crates/playback-engine/Cargo.toml:6-7 (stale scope comment; lib.rs is the audited file but the contradiction is with its own module list)
Code:
```text
# states, two-level session epochs, and the trait-level pipeline wiring
# demux → packet queues → decoders → sinks. Seek/sync are deliberately
```
What is wrong: the manifest header still says "Seek/sync are deliberately EXCLUDED (Phase 3: cards 3.A1-3.A3, 3.B1)" — but the crate contains seek.rs (lib.rs:103), tracks.rs, frame_step.rs, and the lib.rs doc itself announces the seek flow (34-36) and track switching (43-45).
Why it matters: manifest comments are read for dependency archaeology; a stale "excluded" claim for code that shipped invites wrong conclusions about the crate's scope (the same class as F-001, one level down).
What correct looks like: update the header to the current scope or delete the scope sentence.
Confidence: HIGH — Verification: both files read.

## 4. Per-function table (module declarations and re-export groups)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| crate doc (what/why/tour/acceptance/conventions) | 1-91 | REAL | S3 | F-001 (stale counts), F-002 (tour gaps); acceptance pointers all verified real |
| `pub mod` × 15 | 93-107 | REAL | S4 | All modules exist as files; 3 undocumented in tour (F-002) |
| re-exports: chapters/epoch/error/event | 109-117 | REAL | — | Compile-verified (baseline build CLEAN) |
| re-exports: events/frame_step/hotplug | 118-125 | REAL | — | HotplugStats/HotplugStep re-exported — the unwired surface my hotplug.rs audit flags is also the crate's public API |
| re-exports: pipeline | 126-130 | REAL | — | Queue/ceiling/budget constants — the shipped lane's knobs |
| re-exports: recovery | 131-134 | REAL | — | SourceRetryGuard/SurfaceGuard re-exported despite zero production callers (recovery part2 F-001) |
| re-exports: seek/session/state/tracks | 135-145 | REAL | — | The state machine's public surface |

## 5. Test honesty
This file has no tests of its own; its claims point at suites:
| Referenced suite | Exists? | Green? |
|---|---|---|
| tests/table.rs (104-pair classification, wire-conformance, guard-only-legal-inputs) | YES | YES (no-fail-fast baseline) |
| tests/session_flow.rs (staleness proofs) | YES | YES |
| tests/seek_flow.rs (seven steps, D4 rollback) | YES | YES |
| tests/track_switch.rs (multi-track fixture) | YES | YES — fixture verified real |
| tests/model_check.rs (33 rows, 104 pairs) | YES | YES — and it is what falsifies the doc's 26/29 (F-001) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "TRANSITIONS — 26 rows" | lib.rs:9 | FALSE | 33 rows; state.rs:972 + model_check.rs:312 assert it |
| "29 legal rows, 75 typed rejections" | lib.rs:63 | FALSE | 33 legal / 71 rejected (104 − 33) |
| "8 wire-visible states" | lib.rs:7 | REAL | SessionState: 8 variants |
| "Std-only, zero external dependencies, no unsafe" | lib.rs:88 | REAL | Cargo.toml: 6 internal path deps; only "unsafe" match in src is this sentence |
| "track switching landed with card 3.A3 and source replacement (3.A2) IS here" | lib.rs:52-55 | REAL | tracks.rs, snapshot.rs, tests/replacement.rs, tests/source_swap.rs exist |
| "Seek/sync are deliberately EXCLUDED (Phase 3)" | Cargo.toml:6-7 | FALSE (stale) | seek.rs + seek_flow.rs shipped (F-003) |

## 7. Dead / duplicated / unreachable code
- Nothing dead in this file; the re-export of test-only-reachable types (SourceRetryGuard, SurfaceGuard, HotplugStep) is a public-API decision, flagged in the respective module audits.

## 8. Dependencies & flags
- 6 internal deps, testkit + demux + codecs + input as dev-deps (documented dev-edge pattern with the Lead flag in the manifest comment — legitimate).
- `[lints] workspace = true` — the workspace deny-lists apply; baseline clippy CLEAN.

## 9. Open questions for the next agent
- Q-1 (doc-wave owner): when F-001's counts are fixed, check every other crate-root doc for the same stale-hard-number pattern (this file had two).
- Q-2 (state.rs owner, W03 peer): confirm the 33-row table covers exactly doc 05 §3.3's legal edges — the wire-conformance test (table.rs) is the right place; note lib.rs:9's "every one a legal doc 05 §3.3 wire transition" claim rides on it.
