# AUDIT: crates/playback-engine/src/lib.rs

| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-15 (static analysis; no cargo per adaptations) |
| Lines / functions | 146 / 15 `pub mod` declarations + 13 `pub use` groups (no executable functions) |
| File verdict | REAL-UNPROVEN (crate root; the doc header carries stale counts) |
| Reachable from shipped entry point? | **YES** — the crate is the engine every host drives (`Engine::open` re-exported at 136-140; apps/cli/src/host.rs constructs it). |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:1 |
| Commands run | rg `unsafe` in crates/playback-engine/src (1 hit = the doc claim itself); cross-check of row counts (lib.rs:9 "26 rows", lib.rs:63 "29 legal rows", state.rs:972 `assert_eq!(TRANSITIONS.len(), 33)`); Cargo.toml read |

## 1. One-paragraph verdict

The crate root: a long honest module map, 15 modules, and the re-export surface. The engine itself (session/state/seek/tracks/pipeline/epoch/event) is real and shipped; five of the fifteen modules I audited or cross-referenced are dead code in every shipped binary (bookmarks, frame_step, recovery, hotplug's hooks, and media-clock's backends — the last in another crate). The findings here are documentation drift on the crate's own flagship invariant: the header claims the transition table has "26 rows" at line 9 and "29 legal rows" at line 63, while the test in state.rs pins **33** — three different numbers for one const array, two of them wrong in the same file. The "std-only, zero external dependencies, no unsafe" convention claim is verified true (all deps are workspace-internal path deps; zero `unsafe` blocks in src/).

## 2. What actually works (verified)

- The re-export surface is complete and matches the audited modules (state items at 141-144, seek at 135, recovery at 131-134, frame_step at 122-124, bookmarks at 93, hotplug at 125).
- "no unsafe" (line 88): verified — the only `unsafe` token in `crates/playback-engine/src/` is this doc line itself.
- "zero external dependencies": verified — `[dependencies]` in Cargo.toml are six workspace-internal path crates; dev-dependencies (demux/codecs/input/testkit) are test-only, consistent with the C5 note.

## 3. Findings

### F-001 [S3] — The crate header misstates the transition table's size twice, disagreeing with its own crate's test-pinned truth

Where: crates/playback-engine/src/lib.rs:9-10 and 62-63 vs crates/playback-engine/src/state.rs:966-973.

Code (lib.rs:9-10, verbatim):
```rust
//! the transition table as **data** ([`TRANSITIONS`] — 26 rows,
//! every one a legal doc 05 §3.3 wire transition). Around it:
```

Code (lib.rs:62-64, verbatim):
```rust
//!   integration suite `tests/table.rs` classifies **every**
//!   `(state, input)` pair (8 × 13 = 104: 29 legal rows, 75 typed
//!   rejections), proves every row is wire-conformant against
```

Code (state.rs:966-973, verbatim):
```rust
        // 33 rows (26 of 2.C2 + 3.C1's four: Opening+Stop,
        // Buffering+Pause, Buffering+Stop, Paused+Primed + 3.A3's
        // three SwitchTrack work-carriers); adding one is a
        // deliberate act (worklog + doc 03 §4.6 reference in the
        // row's comment).
        assert_eq!(TRANSITIONS.len(), 33);
```

What is wrong: three counts for one array — 26 (lib.rs:9), 29 (lib.rs:63), 33 (the test, and the actual row count). Note also state.rs's own header says "30 rows" and "8 × 12 = 96" (my state.rs audit F-003/F-004) while lib.rs says 8 × 13 = 104. The doc rot is symmetric and self-contradicting across the crate: a reader cannot learn the table's size from any prose; only the test tells the truth.

Why it matters: the transition table is the crate's central data structure and its doc-claims are the basis of the wire-conformance story; four wrong prose numbers across two files is exactly the documentation-drift pattern that let the broader docs diverge from code. S3 (misleading crate-level documentation of a correctness-critical invariant), not S4, because downstream auditors citing the header would mis-report the machine.

Confidence: HIGH — Verification: read + grep count (33 `Transition {` rows in state.rs).

### F-002 [S4] — The header's "What is deliberately NOT here (Phase 3+)" section is stale relative to the module list below it

Where: crates/playback-engine/src/lib.rs:49-57.

Code (49-53, verbatim):
```rust
//! # What is deliberately NOT here (Phase 3+)
//!
//! A/V sync, rate, the thread-per-session model and the event
//! broadcast bus are excluded by the card (3.B1, 3.B3, 2.E1); track
```

What is wrong: the section then immediately concedes "track switching landed with card 3.A3 and source replacement (3.A2) IS here" — while `seek` (card 3.A1, also Phase 3) is both a module (line 103) and described in the AC section (74-78). The 2.C2-era "not here" list was partially updated in place, leaving a self-contradicting header: "seek is excluded" is nowhere literally claimed but the Cargo.toml header (my next file) still says exactly that. A/V sync *is* still not here (it lives in media-clock/video-engine) — that part is true. Hygiene: rewrite the section as a current-state inventory rather than a patchwork of card-era exclusions.

Confidence: HIGH — Verification: read + cross-file comparison.

## 4. Per-function table

| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| crate docs (module map, AC map, conventions) | 1-92 | REAL-UNPROVEN | S3/S4 | F-001 counts; F-002 stale exclusions |
| `pub mod` declarations ×15 | 93-107 | REAL | — | bookmarks/frame_step/recovery/hotplug are dead-in-shipped (see my per-module audits) |
| re-exports (chapters/epoch/error/event/events/frame_step/hotplug/pipeline/recovery/seek/session/state/tracks) | 109-145 | REAL | — | surface matches modules; `transition_table` re-export at 139 |

## 5. Test honesty

No tests in this file (crate root). The AC section (59-84) maps each acceptance criterion to a real suite (tests/table.rs, session_flow.rs, seek_flow.rs, track_switch.rs) — all exist in `crates/playback-engine/tests/` (verified by directory listing; seek_flow.rs is my file #24).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "TRANSITIONS — 26 rows" | lib.rs:9 | FALSE | 33 rows (test-pinned) |
| "8 × 13 = 104: 29 legal rows, 75 typed rejections" | lib.rs:63 | FALSE (29) / TRUE (104) | 33 legal → 71 rejections; the 8×13 total is correct |
| "Std-only, zero external dependencies, no unsafe" | lib.rs:88 | REAL | verified: path deps only; zero unsafe blocks |
| "the host's `Engine::poll` call IS the session thread" | lib.rs:56-57 | REAL | matches the pump-driven reality (and is the root of the CLI's frozen-clock behavior — my backends.rs audit F-002) |
| "source replacement (3.A2) IS here" | lib.rs:53-54 | REAL | `Engine::open` → `replace_live_session` (session.rs:1071-1073) |

## 7. Dead / duplicated / unreachable code

Modules re-exported but dead in shipped binaries: `bookmarks` (my audit: S0), `frame_step` (S0), `recovery` (S0-reachability), `hotplug` hooks (uncalled). The re-exports themselves are the public API contract — the deadness is architectural, not removable here.

## 8. Dependencies & flags

No feature flags. All six dependencies are workspace-internal (codec-api, commands, media-clock, media-types, packet-queue, plugin-api); dev-deps demux/codecs/input/testkit are test-only edges (boundaries.toml-exempt per the Cargo.toml comment).

## 9. Open questions for the next agent

- tests/table.rs — does it really classify all 104 pairs and check wire conformance (the header's claim, with wrong row counts)? Wave-16 test audit.
- `chapters` and `snapshot` modules — not in my assignment; same reachability question as bookmarks (chapters is cited by the CLI's chapter-seek refusal "hosts resolve chapters to Absolute" — if no host does, the refusal is permanent).
